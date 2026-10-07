//! The match simulation. Fixed 60 Hz step, deterministic for a given input
//! stream and seed. No platform dependencies.

use crate::ai;
use crate::ball::Ball;
use crate::data::{FluxKind, Pos, Stats, TeamDef};
use crate::events::{Restart, SimEvent};
use crate::flux::{self, BaseAction, FluxAction, PoolGain, CHARGE, POOL_MAX};
use crate::input::{InputFrame, KickCmd, KickKind};
use crate::pitch::*;
use crate::player::{Action, ActiveFlux, PlayerState};
use crate::rng::Rng;
use glam::{Vec2, Vec3};

pub const DT: f32 = 1.0 / 60.0;
pub const PLAYERS_PER_TEAM: usize = 7;

#[derive(Clone, Debug)]
pub struct MatchConfig {
    pub half_len_secs: f32,
    pub seed: u64,
    /// 0..=1, AI quality.
    pub difficulty: f32,
    /// Which teams are human controlled.
    pub human: [bool; 2],
}

impl Default for MatchConfig {
    fn default() -> Self {
        MatchConfig { half_len_secs: 180.0, seed: 7, difficulty: 0.6, human: [true, false] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    KickOff { team: usize, t: f32 },
    Play,
    DeadBall { restart: Restart, team: usize, at: Vec2, t: f32, taker: usize },
    Goal { team: usize, t: f32 },
    HalfTime { t: f32 },
    FullTime,
}

#[derive(Clone, Debug)]
pub struct TeamState {
    pub def: TeamDef,
    pub kind: FluxKind,
    pub pool: f32,
    pub score: u32,
    pub human: bool,
    pub controlled: Option<usize>,
    pub team_flux_until: f32,
    /// Sim player ids, goalkeeper first.
    pub players: Vec<usize>,
    pub last_switch: f32,
    pub gk_hold: f32,
    pub gk_shot_seen: Option<f32>,
    pub gk_dive_target: Option<Vec2>,
    pub gk_reach_mult: f32,
}

impl TeamState {
    pub fn charges(&self) -> u32 {
        (self.pool / CHARGE).floor() as u32
    }
    pub fn team_flux_active(&self, now: f32) -> bool {
        self.team_flux_until > now
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cloud {
    pub p: Vec2,
    pub r: f32,
    pub team: usize,
    pub t: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AiMemory {
    pub until: f32,
    pub frame: InputFrame,
}

#[derive(Clone, Debug)]
pub struct Sim {
    pub cfg: MatchConfig,
    pub tick: u64,
    /// Total simulated seconds (slow motion included).
    pub time: f32,
    /// Match clock within the current half, seconds.
    pub clock: f32,
    pub half: u8,
    pub phase: Phase,
    pub ball: Ball,
    pub players: Vec<PlayerState>,
    pub teams: Vec<TeamState>,
    pub clouds: Vec<Cloud>,
    pub events: Vec<SimEvent>,
    pub rng: Rng,
    pub slowmo: u32,
    pub ai_mem: Vec<AiMemory>,
    /// (team, target) of the last pass, for the completion bonus.
    pub pending_pass: Option<(usize, usize, usize)>,
    pub side_dir: [f32; 2],
}

fn n(v: u8) -> f32 {
    Stats::n(v)
}

fn clamp_pitch(p: Vec2, margin: f32) -> Vec2 {
    Vec2::new(
        p.x.clamp(-HALF_LEN - margin, HALF_LEN + margin),
        p.y.clamp(-HALF_WID - margin, HALF_WID + margin),
    )
}

fn rotate(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

impl Sim {
    pub fn new(home: TeamDef, away: TeamDef, cfg: MatchConfig) -> Sim {
        let mut players = Vec::new();
        let mut teams = Vec::new();
        for (t, def) in [home, away].into_iter().enumerate() {
            let starters = def.starters();
            let mut ids = Vec::new();
            for (slot, &di) in starters.iter().enumerate() {
                let pd = &def.players[di];
                let id = t * PLAYERS_PER_TEAM + slot;
                ids.push(id);
                players.push(PlayerState {
                    id,
                    team: t,
                    def_index: di,
                    name: pd.name.clone(),
                    number: pd.number,
                    pos: pd.pos,
                    native: pd.native,
                    stats: pd.stats,
                    p: Vec2::ZERO,
                    v: Vec2::ZERO,
                    facing: Vec2::X,
                    stamina: 1.0,
                    strain: 0.0,
                    burned_out: false,
                    sick: false,
                    sick_timer: 0.0,
                    action: Action::Idle,
                    flux: None,
                    flux_cooldown: 0.0,
                    hold_pass: 0.0,
                    hold_lob: 0.0,
                    hold_through: 0.0,
                    hold_shoot: 0.0,
                    sprint_ball_timer: 0.0,
                    press_timer: 0.0,
                    next_decision: 0.0,
                    ai_dribble_side: 1.0,
                });
            }
            teams.push(TeamState {
                kind: def.flux,
                def,
                pool: 0.0,
                score: 0,
                human: cfg.human[t],
                controlled: None,
                team_flux_until: -1.0,
                players: ids,
                last_switch: 0.0,
                gk_hold: 0.0,
                gk_shot_seen: None,
                gk_dive_target: None,
                gk_reach_mult: 1.0,
            });
        }
        let n_players = players.len();
        let mut sim = Sim {
            rng: Rng::new(cfg.seed),
            cfg,
            tick: 0,
            time: 0.0,
            clock: 0.0,
            half: 1,
            phase: Phase::Play,
            ball: Ball::default(),
            players,
            teams,
            clouds: Vec::new(),
            events: Vec::new(),
            slowmo: 0,
            ai_mem: vec![AiMemory::default(); n_players],
            pending_pass: None,
            side_dir: [1.0, -1.0],
        };
        sim.setup_kickoff(0);
        sim
    }

    // ----------------------------------------------------------------- helpers

    #[inline]
    pub fn attack_dir(&self, team: usize) -> f32 {
        self.side_dir[team]
    }

    pub fn opponent(team: usize) -> usize {
        1 - team
    }

    pub fn keeper(&self, team: usize) -> usize {
        self.teams[team].players[0]
    }

    /// World position of a formation anchor.
    pub fn anchor_world(&self, team: usize, attack: bool, slot: usize) -> Vec2 {
        let def = &self.teams[team].def;
        let f = if attack { &def.attack } else { &def.defend };
        let (fx, fy) = f.anchors.get(slot).copied().unwrap_or((0.5, 0.0));
        let dir = self.attack_dir(team);
        Vec2::new(dir * (-HALF_LEN + fx * 2.0 * HALF_LEN), fy * HALF_WID * 0.9 * dir)
    }

    pub fn slot_of(&self, id: usize) -> usize {
        id % PLAYERS_PER_TEAM
    }

    pub fn team_of(&self, id: usize) -> usize {
        self.players[id].team
    }

    pub fn dist_to_ball(&self, id: usize) -> f32 {
        self.players[id].p.distance(self.ball.xy())
    }

    pub fn in_cloud(&self, p: Vec2, team: usize) -> bool {
        self.clouds.iter().any(|c| c.team != team && c.p.distance(p) < c.r)
    }

    pub fn ball_hidden(&self) -> bool {
        self.ball.flags.hidden_until > self.time
    }

    pub fn team_flux_name(&self, team: usize) -> &'static str {
        match self.teams[team].kind {
            FluxKind::Breath => "Akillian Blizzard",
            FluxKind::Smog => "Eclipse",
        }
    }

    pub fn is_dead_ball(&self) -> bool {
        matches!(self.phase, Phase::DeadBall { .. } | Phase::KickOff { .. })
    }

    pub fn dead_ball_taker(&self) -> Option<usize> {
        match self.phase {
            Phase::KickOff { team, .. } => Some(self.teams[team].players[6]),
            Phase::DeadBall { taker, .. } => Some(taker),
            _ => None,
        }
    }

    /// A stable hash of the dynamic state, for determinism tests.
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u32| {
            h ^= v as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for f in [self.ball.p.x, self.ball.p.y, self.ball.p.z, self.ball.v.x, self.ball.v.y] {
            mix(f.to_bits());
        }
        for p in &self.players {
            mix(p.p.x.to_bits());
            mix(p.p.y.to_bits());
            mix(p.strain.to_bits());
        }
        for t in &self.teams {
            mix(t.score);
            mix(t.pool.to_bits());
        }
        mix(self.tick as u32);
        h
    }

    fn gain(&mut self, team: usize, g: PoolGain) {
        let kind = self.teams[team].kind;
        let t = &mut self.teams[team];
        t.pool = (t.pool + flux::pool_gain(kind, g)).min(POOL_MAX);
    }

    // ------------------------------------------------------------ set pieces

    pub fn setup_kickoff(&mut self, team: usize) {
        self.ball.place(Vec2::ZERO);
        self.clouds.clear();
        for t in 0..2 {
            let dir = self.attack_dir(t);
            for slot in 0..PLAYERS_PER_TEAM {
                let id = self.teams[t].players[slot];
                let mut p = self.anchor_world(t, false, slot);
                // Everybody in their own half.
                if p.x * dir > -1.5 {
                    p.x = -1.5 * dir;
                }
                if t == team && slot == 6 {
                    p = Vec2::new(-0.6 * dir, 0.0);
                }
                if t == team && slot == 5 {
                    p = Vec2::new(-2.5 * dir, 1.5 * dir);
                }
                let pl = &mut self.players[id];
                pl.p = p;
                pl.v = Vec2::ZERO;
                pl.facing = Vec2::new(dir, 0.0);
                pl.action = Action::Idle;
                pl.flux = None;
            }
            self.teams[t].gk_shot_seen = None;
            self.teams[t].gk_dive_target = None;
            if self.teams[t].human {
                self.teams[t].controlled = Some(self.teams[t].players[6]);
            }
        }
        self.phase = Phase::KickOff { team, t: 0.0 };
        self.events.push(SimEvent::Whistle);
    }

    fn set_dead_ball(&mut self, restart: Restart, team: usize, at: Vec2) {
        let at = clamp_pitch(at, 0.0);
        self.ball.place(at);
        let taker = if restart == Restart::GoalKick {
            self.keeper(team)
        } else {
            let mut best = self.teams[team].players[1];
            let mut bd = f32::MAX;
            for &id in &self.teams[team].players[1..] {
                let d = self.players[id].p.distance(at);
                if d < bd {
                    bd = d;
                    best = id;
                }
            }
            best
        };
        if restart == Restart::Penalty {
            let dir = self.attack_dir(team);
            let opp = Self::opponent(team);
            let gk = self.keeper(opp);
            self.players[gk].p = Vec2::new(goal_x(dir) - dir * 0.5, 0.0);
            self.players[gk].v = Vec2::ZERO;
            for id in 0..self.players.len() {
                if id == gk || id == taker {
                    continue;
                }
                if in_box(self.players[id].p, dir) {
                    let p = &mut self.players[id].p;
                    p.x = goal_x(dir) - dir * (BOX_DEPTH + 1.5);
                }
            }
        }
        for p in &mut self.players {
            p.v = Vec2::ZERO;
            if !matches!(p.action, Action::Stunned { .. }) {
                p.action = Action::Idle;
            }
        }
        self.phase = Phase::DeadBall { restart, team, at, t: 0.0, taker };
        if self.teams[team].human {
            self.teams[team].controlled = Some(taker);
        }
        self.events.push(SimEvent::OutOfPlay { restart, team, at });
    }

    // ------------------------------------------------------------------ step

    pub fn step(&mut self, human: [Option<InputFrame>; 2]) {
        self.events.clear();
        let dt = if self.slowmo > 0 {
            self.slowmo -= 1;
            DT * 0.25
        } else {
            DT
        };
        self.tick += 1;
        self.time += dt;

        match self.phase {
            Phase::FullTime => return,
            Phase::Goal { team, t } => {
                let t = t + dt;
                if t > 3.0 {
                    self.setup_kickoff(Self::opponent(team));
                } else {
                    self.phase = Phase::Goal { team, t };
                }
                self.decay_players(dt);
                return;
            }
            Phase::HalfTime { t } => {
                let t = t + dt;
                if t > 3.0 {
                    self.half = 2;
                    self.clock = 0.0;
                    self.side_dir = [-1.0, 1.0];
                    for p in &mut self.players {
                        p.strain = 0.0;
                        p.burned_out = false;
                        p.sick = false;
                        p.stamina = (p.stamina + 0.4).min(1.0);
                    }
                    self.setup_kickoff(1);
                } else {
                    self.phase = Phase::HalfTime { t };
                }
                return;
            }
            Phase::KickOff { team, t } => self.phase = Phase::KickOff { team, t: t + dt },
            Phase::DeadBall { restart, team, at, t, taker } => {
                self.phase = Phase::DeadBall { restart, team, at, t: t + dt, taker }
            }
            Phase::Play => {}
        }
        self.clock += dt;
        self.update_timers(dt);

        let frames = self.gather_inputs(human);
        for id in 0..self.players.len() {
            self.apply_player(id, &frames[id], dt);
        }
        self.separate_players();
        self.update_ball(dt);
        self.update_keepers(dt);
        self.rules();
        self.update_switching(&human);

        if self.clock >= self.cfg.half_len_secs && matches!(self.phase, Phase::Play) {
            if self.half == 1 {
                self.phase = Phase::HalfTime { t: 0.0 };
                self.events.push(SimEvent::HalfTime);
            } else {
                self.phase = Phase::FullTime;
                self.events.push(SimEvent::FullTime);
            }
        }
    }

    fn decay_players(&mut self, dt: f32) {
        for p in &mut self.players {
            p.v *= 1.0 - 8.0 * dt;
            if let Some(f) = &mut p.flux {
                f.remaining -= dt;
                if f.remaining <= 0.0 {
                    p.flux = None;
                }
            }
        }
    }

    fn update_timers(&mut self, dt: f32) {
        for c in &mut self.clouds {
            c.t -= dt;
        }
        self.clouds.retain(|c| c.t > 0.0);
        for t in &mut self.teams {
            // A full pool leaks slowly so hoarding has a cost.
            if t.pool >= POOL_MAX - 0.01 {
                t.pool -= 2.0 * dt;
            }
            t.gk_reach_mult = 1.0;
        }
        for p in &mut self.players {
            p.flux_cooldown -= dt;
            if let Some(f) = &mut p.flux {
                f.remaining -= dt;
                if f.remaining <= 0.0 {
                    p.flux = None;
                    p.flux_cooldown = 0.5;
                }
            }
            if !p.burned_out {
                p.strain = (p.strain - 1.5 * dt).max(0.0);
            }
            if p.sick {
                p.sick_timer -= dt;
            }
        }
    }

    fn gather_inputs(&mut self, human: [Option<InputFrame>; 2]) -> Vec<InputFrame> {
        let mut frames = vec![InputFrame::default(); self.players.len()];
        let mut rng = self.rng.clone();
        for id in 0..self.players.len() {
            let team = self.players[id].team;
            let controlled = self.teams[team].human && self.teams[team].controlled == Some(id);
            if controlled && !self.players[id].is_gk()
                && let Some(f) = human[team] {
                    frames[id] = f;
                    continue;
                }
            let mem = self.ai_mem[id];
            if mem.until > self.time {
                frames[id] = mem.frame;
                // A kick fires once.
                self.ai_mem[id].frame.kick = None;
                // Movement is refreshed every tick so chasing stays smooth.
                frames[id].movement = ai::movement_only(self, id, mem.frame.movement);
            } else {
                let (frame, hold) = ai::decide(self, &mut rng, id);
                frames[id] = frame;
                let mut stored = frame;
                stored.kick = None;
                self.ai_mem[id] = AiMemory { until: self.time + hold, frame: stored };
            }
        }
        self.rng = rng;
        frames
    }

    // --------------------------------------------------------------- players

    fn apply_player(&mut self, id: usize, inp: &InputFrame, dt: f32) {
        let team = self.players[id].team;
        let now = self.time;

        // Sickness stumbles.
        if self.players[id].sick && self.players[id].sick_timer <= 0.0 {
            self.players[id].sick_timer = 3.0 + self.rng.f32() * 3.0;
            if self.players[id].action.can_act() {
                self.players[id].action = Action::Stunned { t: 0.0 };
            }
        }

        // Action timers.
        let action = self.players[id].action;
        match action {
            Action::Idle => {}
            Action::Kick { t } => {
                self.players[id].action = if t > 0.3 { Action::Idle } else { Action::Kick { t: t + dt } };
            }
            Action::Tackle { t, dir, flux, resolved } => {
                if !resolved && t >= 0.08 {
                    let ok = self.resolve_tackle(id, dir, flux, false);
                    self.players[id].action = if ok {
                        Action::Kick { t: 0.1 }
                    } else {
                        Action::Tackle { t: t + dt, dir, flux, resolved: true }
                    };
                } else if t > 0.45 {
                    self.players[id].action = Action::Stunned { t: 0.3 };
                } else {
                    self.players[id].action = Action::Tackle { t: t + dt, dir, flux, resolved };
                }
            }
            Action::Slide { t, dir, resolved } => {
                self.players[id].v = dir * 7.5;
                let mut res = resolved;
                if !resolved && self.dist_to_ball(id) < 1.0 {
                    let ok = self.resolve_tackle(id, dir, false, true);
                    res = true;
                    if ok {
                        self.players[id].action = Action::Stunned { t: 0.25 };
                        return;
                    }
                }
                self.players[id].action =
                    if t > 0.45 { Action::Stunned { t: 0.0 } } else { Action::Slide { t: t + dt, dir, resolved: res } };
            }
            Action::Stunned { t } => {
                self.players[id].action = if t > 0.55 { Action::Idle } else { Action::Stunned { t: t + dt } };
            }
            Action::Frozen { t } => {
                self.players[id].action = if t > 0.6 { Action::Idle } else { Action::Frozen { t: t + dt } };
            }
            Action::WindUp { t, action: fa, dir, kick } => {
                if t + dt >= fa.spec().wind_up {
                    self.players[id].action = Action::Idle;
                    self.activate_flux(id, fa, dir, kick);
                } else {
                    self.players[id].action = Action::WindUp { t: t + dt, action: fa, dir, kick };
                }
            }
            Action::Dive { t } => {
                self.players[id].action = if t > 0.8 { Action::Idle } else { Action::Dive { t: t + dt } };
            }
            Action::Jump { t } => {
                self.players[id].action = if t > 0.6 { Action::Idle } else { Action::Jump { t: t + dt } };
            }
        }

        // Dead-ball overrides.
        let dead = self.is_dead_ball();
        let taker = self.dead_ball_taker();
        let mut mv = inp.movement;
        let mut sprint = inp.sprint.held;
        if dead {
            if taker == Some(id) {
                let d = self.ball.xy() - self.players[id].p;
                if d.length() > 0.9 {
                    mv = d.normalize();
                    sprint = true;
                } else {
                    mv = Vec2::ZERO;
                    if self.ball.owner != Some(id) {
                        self.ball.owner = Some(id);
                        self.ball.last_touch = Some(id);
                        self.ball.last_team = Some(team);
                    }
                }
            } else {
                mv = Vec2::ZERO;
                sprint = false;
            }
        }

        // Flux triggers.
        let has_ball = self.ball.owner == Some(id);
        if !dead && self.players[id].action.can_act() {
            let kind = self.teams[team].kind;
            if inp.flux && inp.sprint.pressed && mv.length() > 0.1 {
                let fa = flux::map(kind, BaseAction::Sprint);
                self.try_start_flux(id, fa, mv.normalize(), None);
            }
            if inp.special.pressed {
                let fa = flux::map(kind, BaseAction::Special);
                let dir = if mv.length() > 0.1 { mv.normalize() } else { self.players[id].facing };
                self.try_start_flux(id, fa, dir, None);
            }
            if inp.team_flux.pressed && self.teams[team].pool >= POOL_MAX - 0.5 && !self.teams[team].team_flux_active(now) {
                self.teams[team].pool = 0.0;
                self.teams[team].team_flux_until = now + flux::TEAM_FLUX_DURATION;
                self.events.push(SimEvent::TeamFlux { team });
            }
        }

        // Movement.
        {
            let opp_eclipse = self.teams[Self::opponent(team)].team_flux_active(now)
                && self.teams[Self::opponent(team)].kind == FluxKind::Smog;
            let in_cloud = self.in_cloud(self.players[id].p, team);
            let far_from_ball = self.dist_to_ball(id) > 8.0;
            let pl = &mut self.players[id];
            let locked = pl.action.locked();
            if locked {
                mv = Vec2::ZERO;
            }
            let mut mult = 1.0;
            if pl.flux_active(FluxAction::Burst) {
                mult *= 1.8;
            }
            if pl.burned_out {
                mult *= 0.75;
            } else if pl.fatigued() {
                mult *= 0.85;
            }
            if pl.sick {
                mult *= 0.9;
            }
            if in_cloud {
                mult *= 0.5;
            }
            if opp_eclipse && far_from_ball {
                mult *= 0.7;
            }
            if has_ball {
                mult *= 0.88 + 0.1 * n(pl.stats.dribbling);
            }
            if pl.stamina < 0.3 {
                mult *= 0.75 + pl.stamina * 0.8;
            }
            let base = 5.2 + 3.2 * n(pl.stats.pace);
            let max_speed = base * if sprint { 1.35 } else { 1.0 } * mult;
            let desired = mv.clamp_length_max(1.0) * max_speed;
            let accel = (11.0 + 15.0 * n(pl.stats.acceleration)) * if pl.flux_active(FluxAction::Burst) { 2.0 } else { 1.0 };
            if !matches!(pl.action, Action::Slide { .. } | Action::Dive { .. }) {
                let dv = desired - pl.v;
                pl.v += dv.clamp_length_max(accel * dt);
            }
            pl.p = clamp_pitch(pl.p + pl.v * dt, 1.5);
            if pl.v.length() > 0.6 {
                pl.facing = pl.v.normalize();
            } else if mv.length() > 0.1 {
                pl.facing = mv.normalize();
            }
            // Stamina.
            let drain = if sprint && mv.length() > 0.1 { 0.035 } else if mv.length() > 0.1 { 0.006 } else { -0.02 };
            pl.stamina = (pl.stamina - drain * (1.3 - 0.6 * n(pl.stats.stamina)) * dt).clamp(0.0, 1.0);
        }

        if dead && taker != Some(id) {
            return;
        }

        // Kicks: human hold/release or AI direct command.
        let mut cmd = inp.kick;
        {
            let pl = &mut self.players[id];
            let hold = |b: &crate::input::Button, h: &mut f32, kind: KickKind| -> Option<KickCmd> {
                if b.held {
                    *h += dt;
                }
                if b.released {
                    let power = (*h / 0.8).clamp(0.12, 1.0);
                    *h = 0.0;
                    return Some(KickCmd { kind, power, dir: inp.movement, target: None, flux: inp.flux, manual: inp.manual });
                }
                if !b.held {
                    *h = 0.0;
                }
                None
            };
            if cmd.is_none() {
                cmd = hold(&inp.shoot, &mut pl.hold_shoot, KickKind::Shot)
                    .or_else(|| hold(&inp.pass, &mut pl.hold_pass, KickKind::Pass))
                    .or_else(|| hold(&inp.through, &mut pl.hold_through, KickKind::Through))
                    .or_else(|| hold(&inp.lob, &mut pl.hold_lob, KickKind::Lob));
            }
        }
        if let Some(c) = cmd {
            self.try_kick(id, c);
        }

        // Tackles.
        if !dead && !has_ball && self.players[id].action.can_act() {
            let opp_has = matches!(self.ball.owner, Some(o) if self.players[o].team != team);
            let near = self.dist_to_ball(id);
            if inp.action_a.pressed && (opp_has || self.ball.owner.is_none()) && near < 2.2 {
                let dir = (self.ball.xy() - self.players[id].p).normalize_or(self.players[id].facing);
                let kind = self.teams[team].kind;
                if inp.flux && opp_has {
                    let fa = flux::map(kind, BaseAction::Tackle);
                    if !self.try_start_flux(id, fa, dir, None) {
                        self.players[id].action = Action::Tackle { t: 0.0, dir, flux: false, resolved: false };
                    } else if fa == FluxAction::FrostLock {
                        self.players[id].action = Action::Tackle { t: 0.0, dir, flux: true, resolved: false };
                    }
                } else {
                    self.players[id].action = Action::Tackle { t: 0.0, dir, flux: false, resolved: false };
                }
            } else if inp.slide.pressed && near < 4.0 {
                let dir = (self.ball.xy() - self.players[id].p).normalize_or(self.players[id].facing);
                self.players[id].action = Action::Slide { t: 0.0, dir, resolved: false };
            } else if inp.flux && inp.action_a.pressed && opp_has && near < 9.0 {
                // Smog Snatch has range: teleport to the carrier.
                let kind = self.teams[team].kind;
                if kind == FluxKind::Smog {
                    let dir = (self.ball.xy() - self.players[id].p).normalize_or(Vec2::X);
                    self.try_start_flux(id, FluxAction::SmogSnatch, dir, None);
                }
            }
        }

        // Pool gains from style of play.
        if has_ball && sprint && mv.length() > 0.1 {
            let pl = &mut self.players[id];
            pl.sprint_ball_timer += dt;
            if pl.sprint_ball_timer >= 1.0 {
                pl.sprint_ball_timer -= 1.0;
                self.gain(team, PoolGain::SprintWithBallSecond);
            }
        }
        if !has_ball
            && let Some(o) = self.ball.owner
                && self.players[o].team != team && self.players[o].p.distance(self.players[id].p) < 3.0 {
                    let pl = &mut self.players[id];
                    pl.press_timer += dt;
                    if pl.press_timer >= 1.0 {
                        pl.press_timer -= 1.0;
                        self.gain(team, PoolGain::PressSecond);
                    }
                }
    }

    fn separate_players(&mut self) {
        let n = self.players.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.players[j].p - self.players[i].p;
                let dist = d.length();
                let min = PLAYER_RADIUS * 2.0;
                if dist < min && dist > 1e-4 {
                    let push = d / dist * (min - dist) * 0.5;
                    self.players[i].p -= push;
                    self.players[j].p += push;
                }
            }
        }
    }

    // ----------------------------------------------------------------- kicks

    fn ball_reachable(&self, id: usize) -> bool {
        if self.ball.owner == Some(id) {
            return true;
        }
        if self.ball.owner.is_some() {
            return false;
        }
        self.dist_to_ball(id) < 1.1 && self.ball.p.z < 1.3
    }

    fn try_kick(&mut self, id: usize, cmd: KickCmd) {
        if !self.players[id].action.can_act() && !matches!(self.players[id].action, Action::Kick { .. }) {
            return;
        }
        if !self.ball_reachable(id) {
            return;
        }
        let team = self.players[id].team;
        if cmd.flux && !self.is_dead_ball() {
            let base = match cmd.kind {
                KickKind::Shot => BaseAction::Shoot,
                KickKind::Lob => BaseAction::Lob,
                _ => BaseAction::Pass,
            };
            let fa = flux::map(self.teams[team].kind, base);
            let dir = if cmd.dir.length() > 0.1 { cmd.dir.normalize() } else { self.players[id].facing };
            if self.try_start_flux(id, fa, dir, Some(cmd)) {
                return;
            }
        }
        self.execute_kick(id, cmd, None);
    }

    pub fn pick_pass_target(&self, id: usize, aim: Vec2, kind: KickKind) -> Option<usize> {
        let me = &self.players[id];
        let team = me.team;
        let dir = self.attack_dir(team);
        let mut best = None;
        let mut best_score = f32::MIN;
        for &t in &self.teams[team].players {
            if t == id {
                continue;
            }
            let other = &self.players[t];
            let lead = if kind == KickKind::Through { 0.9 } else { 0.3 };
            let tp = other.p + other.v * lead;
            let d = tp - me.p;
            let dist = d.length();
            if dist < 1.5 {
                continue;
            }
            let dn = d / dist;
            let cos = if aim.length() > 0.1 { dn.dot(aim.normalize()) } else { dn.dot(me.facing) };
            if cos < 0.1 {
                continue;
            }
            let mut blocked = 0.0;
            for &o in &self.teams[Self::opponent(team)].players {
                let op = self.players[o].p;
                let t = ((op - me.p).dot(dn) / dist).clamp(0.0, 1.0);
                let closest = me.p + dn * (t * dist);
                if t > 0.05 && t < 0.95 && closest.distance(op) < 1.3 {
                    blocked += 1.0;
                }
            }
            let progress = (tp.x - me.p.x) * dir / 30.0;
            let score = cos * 2.0 - dist / 35.0 - blocked * 1.2 + progress * 0.3 - if other.is_gk() { 0.8 } else { 0.0 };
            if score > best_score {
                best_score = score;
                best = Some(t);
            }
        }
        best
    }

    fn execute_kick(&mut self, id: usize, cmd: KickCmd, flux_action: Option<FluxAction>) {
        let (p, facing, stats, team) = {
            let m = &self.players[id];
            (m.p, m.facing, m.stats, m.team)
        };
        let dir_sign = self.attack_dir(team);
        let aim = if cmd.dir.length() > 0.1 { cmd.dir.normalize() } else { facing };
        let in_cloud = self.in_cloud(p, team);
        let ai_err = if self.teams[team].human { 1.0 } else { 1.0 + (1.0 - self.cfg.difficulty) * 0.6 };
        let fatigue_err = if self.players[id].fatigued() { 1.4 } else { 1.0 };
        let sick_err = if self.players[id].sick { 1.5 } else { 1.0 };
        let power = cmd.power.clamp(0.1, 1.0);
        let mut target = None;
        let mut v3;
        let mut spin = 0.0;
        match cmd.kind {
            KickKind::Pass | KickKind::Through => {
                let through = cmd.kind == KickKind::Through;
                target = if cmd.manual { None } else { cmd.target.or_else(|| self.pick_pass_target(id, aim, cmd.kind)) };
                let (dirv, speed) = if let Some(t) = target {
                    let o = &self.players[t];
                    let mut tp = o.p + o.v * if through { 0.8 } else { 0.25 };
                    if through {
                        tp.x += dir_sign * (3.0 + 7.0 * power);
                        tp = clamp_pitch(tp, -1.0);
                    }
                    let d = tp - p;
                    let dist = d.length().max(0.5);
                    let auto = if through { (dist * 1.15 + 4.0).clamp(9.0, 26.0) } else { (dist * 1.1 + 3.0).clamp(7.0, 24.0) };
                    (d / dist, auto * (0.85 + 0.3 * power))
                } else {
                    (aim, 8.0 + 15.0 * power)
                };
                let stat = if through { stats.long_pass } else { stats.short_pass };
                let err = ((1.0 - n(stat)) * 0.14 + if in_cloud { 0.25 } else { 0.0 }) * ai_err * fatigue_err * sick_err;
                let dirv = rotate(dirv, err * self.rng.sym());
                v3 = Vec3::new(dirv.x * speed, dirv.y * speed, 0.0);
            }
            KickKind::Lob => {
                target = if cmd.manual { None } else { cmd.target.or_else(|| self.pick_pass_target(id, aim, KickKind::Lob)) };
                let tp = if let Some(t) = target {
                    let o = &self.players[t];
                    o.p + o.v * 0.6 + Vec2::new(dir_sign * 1.5, 0.0)
                } else {
                    p + aim * (10.0 + 22.0 * power)
                };
                let d = tp - p;
                let dist = d.length().max(1.0);
                let dirv = d / dist;
                let t_flight = (dist / 13.0).clamp(0.6, 1.9);
                let vxy = dist / t_flight;
                let vz = 9.81 * t_flight / 2.0;
                let err = (1.0 - n(stats.long_pass)) * 0.16 * ai_err * fatigue_err * sick_err;
                let dirv = rotate(dirv, err * self.rng.sym());
                let vxy = vxy * (1.0 + err * self.rng.sym());
                v3 = Vec3::new(dirv.x * vxy, dirv.y * vxy, vz);
            }
            KickKind::Shot => {
                let gx = goal_x(dir_sign);
                let aim_y = if cmd.manual {
                    // Raw: shoot where the stick points.
                    let far = p + aim * 60.0;
                    far.y.clamp(-GOAL_HALF_WIDTH * 2.0, GOAL_HALF_WIDTH * 2.0)
                } else {
                    (cmd.dir.y * dir_sign).clamp(-1.0, 1.0) * GOAL_HALF_WIDTH * 0.8
                };
                let tp = Vec2::new(gx, aim_y);
                let d = tp - p;
                let dist = d.length().max(1.0);
                let dirv = d / dist;
                let flux_mult = if flux_action.is_some() { 1.5 } else { 1.0 };
                let speed = (15.0 + 15.0 * power) * flux_mult;
                let body = facing.dot(dirv).clamp(-1.0, 1.0);
                let mut err = 0.025 + (1.0 - n(stats.shooting)) * 0.09 + power * 0.04 + (1.0 - body) * 0.08;
                if dist > 22.0 {
                    err += 0.03;
                }
                if flux_action.is_some() {
                    err *= 0.7;
                }
                err *= ai_err * fatigue_err * sick_err;
                let dirv = rotate(dirv, err * self.rng.sym());
                let lift = speed * (0.05 + 0.15 * power) * (1.0 + err * 6.0 * self.rng.sym());
                spin = facing.perp_dot(dirv) * 3.0;
                v3 = Vec3::new(dirv.x * speed, dirv.y * speed, lift.max(0.0));
            }
        }
        if self.players[id].is_gk() && cmd.kind != KickKind::Shot {
            v3 *= 1.05;
        }

        let b = &mut self.ball;
        b.v = v3;
        b.spin = spin;
        b.owner = None;
        b.kicked_by = Some(id);
        b.since_kick = 0.0;
        b.last_touch = Some(id);
        b.last_team = Some(team);
        b.flags = Default::default();
        let now = self.time;
        if let Some(fa) = flux_action {
            match fa {
                FluxAction::AkillianStrike => b.flags.flux_shot = true,
                FluxAction::SmogShot => {
                    b.flags.flux_shot = true;
                    b.flags.hidden_until = now + 0.5;
                }
                FluxAction::IceLane => b.flags.ice_lane = true,
                FluxAction::VeiledPass => {
                    let flight = (v3.length() / 1.0).max(1.0);
                    b.flags.hidden_until = now + (flight / v3.length().max(1.0) * 0.6).clamp(0.3, 1.2);
                }
                FluxAction::PhaseLob => b.flags.phase = true,
                FluxAction::HighBreath => b.flags.high_breath = true,
                _ => {}
            }
        }
        self.players[id].action = Action::Kick { t: 0.0 };
        self.players[id].hold_pass = 0.0;
        self.players[id].hold_shoot = 0.0;
        self.players[id].hold_lob = 0.0;
        self.players[id].hold_through = 0.0;
        self.events.push(SimEvent::Kick { player: id, power, kind: cmd.kind, flux: flux_action.is_some() });
        self.pending_pass = target.map(|t| (team, id, t));
        if cmd.kind != KickKind::Shot
            && let Some(t) = target
                && self.teams[team].human {
                    self.teams[team].controlled = Some(t);
                    self.teams[team].last_switch = now;
                }
        if self.is_dead_ball() {
            if let Phase::KickOff { team: kt, .. } = self.phase {
                self.events.push(SimEvent::KickOff { team: kt });
            }
            self.phase = Phase::Play;
        }
    }

    // --------------------------------------------------------------- tackles

    /// Returns true when the ball was won.
    fn resolve_tackle(&mut self, id: usize, dir: Vec2, flux: bool, slide: bool) -> bool {
        let Some(victim) = self.ball.owner else {
            // Free ball: a tackle is just a strong touch.
            if self.dist_to_ball(id) < 1.0 && self.ball.p.z < 1.2 {
                let b = &mut self.ball;
                b.v = Vec3::new(dir.x * 6.0, dir.y * 6.0, 0.5);
                b.owner = None;
                b.kicked_by = Some(id);
                b.since_kick = 0.0;
                b.last_touch = Some(id);
                b.last_team = Some(self.players[id].team);
                return true;
            }
            return false;
        };
        let team = self.players[id].team;
        if self.players[victim].team == team {
            return false;
        }
        let range = if slide { 1.3 } else { 1.5 };
        if self.players[id].p.distance(self.ball.xy()) > range {
            return false;
        }
        let me = self.players[id].stats;
        let vs = self.players[victim].stats;
        let to_victim = (self.players[victim].p - self.players[id].p).normalize_or(dir);
        let behind = self.players[victim].facing.dot(to_victim) > 0.4; // approaching from behind
        let front = self.players[victim].facing.dot(to_victim) < -0.4;
        let mut p = 0.38 + 0.55 * (n(me.tackle) - n(vs.ball_control) * 0.55 - n(vs.strength) * 0.2);
        if front {
            p += 0.15;
        }
        if behind {
            p -= 0.2;
        }
        if flux {
            p += 0.2;
        }
        if slide {
            p += 0.1;
        }
        if self.players[victim].flux.is_some() {
            p -= 0.25;
        }
        if self.players[victim].sick {
            p += 0.15;
        }
        let p = p.clamp(0.08, 0.92);
        let foul_p = if behind { if flux { 1.0 } else if slide { 0.55 } else { 0.35 } } else if slide { 0.2 } else { 0.08 };
        if self.rng.chance(p) {
            let b = &mut self.ball;
            b.owner = None;
            b.v = Vec3::new(dir.x * 3.0 + self.players[id].facing.x, dir.y * 3.0 + self.players[id].facing.y, 0.3);
            b.kicked_by = Some(id);
            b.since_kick = 0.2;
            b.last_touch = Some(id);
            b.last_team = Some(team);
            if flux && self.players[id].flux_active(FluxAction::FrostLock) {
                self.players[victim].action = Action::Frozen { t: 0.0 };
            } else {
                self.players[victim].action = Action::Stunned { t: 0.15 };
            }
            self.events.push(SimEvent::Tackle { by: id, on: victim, success: true });
            self.gain(team, PoolGain::TackleWon);
            true
        } else {
            self.events.push(SimEvent::Tackle { by: id, on: victim, success: false });
            if self.rng.chance(foul_p) {
                let at = self.players[victim].p;
                let vteam = self.players[victim].team;
                self.events.push(SimEvent::Foul { by: id, on: victim, at });
                self.players[id].action = Action::Stunned { t: 0.0 };
                self.players[victim].action = Action::Stunned { t: 0.0 };
                let dir_v = self.attack_dir(vteam);
                if in_box(at, dir_v) {
                    let spot = Vec2::new(goal_x(dir_v) - dir_v * PENALTY_SPOT, 0.0);
                    self.set_dead_ball(Restart::Penalty, vteam, spot);
                } else {
                    self.set_dead_ball(Restart::FreeKick, vteam, at);
                }
            }
            false
        }
    }

    // ------------------------------------------------------------------ flux

    /// Spends a charge and starts (or winds up) a Flux action. Returns false
    /// if the player or the pool cannot afford it.
    fn try_start_flux(&mut self, id: usize, fa: FluxAction, dir: Vec2, kick: Option<KickCmd>) -> bool {
        let team = self.players[id].team;
        let now = self.time;
        if !self.players[id].can_use_flux() {
            return false;
        }
        let free = self.teams[team].team_flux_active(now);
        let spec = fa.spec();
        let cost = if free { 0.0 } else { spec.cost };
        if self.teams[team].pool < cost {
            return false;
        }
        self.teams[team].pool -= cost;
        // Strain.
        let native = self.players[id].native;
        let mut strain = spec.strain * if free { 0.5 } else { 1.0 };
        if fa.kind() == FluxKind::Smog && !native {
            strain *= 1.5;
        }
        strain *= 1.2 - 0.4 * n(self.players[id].stats.flux_control);
        let pl = &mut self.players[id];
        pl.strain = (pl.strain + strain).min(100.0);
        if pl.strain >= flux::STRAIN_BURNOUT && !pl.burned_out {
            pl.burned_out = true;
            self.events.push(SimEvent::Burnout { player: id });
        }
        if fa.kind() == FluxKind::Smog && !native && pl.strain >= flux::STRAIN_FATIGUE && !pl.sick {
            pl.sick = true;
            pl.sick_timer = 1.0;
            self.events.push(SimEvent::Sick { player: id });
        }
        if fa == FluxAction::Burst {
            pl.stamina = (pl.stamina - 0.15).max(0.0);
        }
        if spec.wind_up > 0.0 {
            pl.action = Action::WindUp { t: 0.0, action: fa, dir, kick };
            pl.v *= 0.3;
            self.events.push(SimEvent::FluxStart { player: id, action: fa });
        } else {
            self.activate_flux(id, fa, dir, kick);
        }
        true
    }

    fn activate_flux(&mut self, id: usize, fa: FluxAction, dir: Vec2, kick: Option<KickCmd>) {
        let now = self.time;
        let team = self.players[id].team;
        let spec = fa.spec();
        self.players[id].flux = Some(ActiveFlux { action: fa, remaining: spec.duration.max(0.1), started_at: now });
        if spec.wind_up == 0.0 {
            self.events.push(SimEvent::FluxStart { player: id, action: fa });
        }
        if self.check_duel(id) {
            // Lost the duel: the action fizzles.
            if self.players[id].flux.is_none() {
                return;
            }
        }
        match fa {
            FluxAction::Burst | FluxAction::FrostLock | FluxAction::WallOfIce | FluxAction::ShadowKeeper => {}
            FluxAction::SmogStep => {
                let from = self.players[id].p;
                let to = clamp_pitch(from + dir * 6.0, 0.0);
                self.players[id].p = to;
                self.events.push(SimEvent::Teleport { player: id, from, to });
                if self.ball.owner == Some(id) {
                    self.ball.p = Vec3::new(to.x + dir.x * 0.5, to.y + dir.y * 0.5, BALL_RADIUS);
                    self.ball.prev = self.ball.p;
                }
            }
            FluxAction::AkillianStrike
            | FluxAction::SmogShot
            | FluxAction::IceLane
            | FluxAction::VeiledPass
            | FluxAction::HighBreath
            | FluxAction::PhaseLob => {
                if let Some(c) = kick {
                    if self.ball_reachable(id) {
                        self.execute_kick(id, c, Some(fa));
                    } else {
                        self.players[id].flux = None;
                        self.teams[team].pool = (self.teams[team].pool + spec.cost * 0.5).min(POOL_MAX);
                        self.events.push(SimEvent::FluxFizzle { player: id });
                    }
                }
            }
            FluxAction::SmogSnatch => {
                if let Some(o) = self.ball.owner {
                    if self.players[o].team != team && self.players[o].p.distance(self.players[id].p) < 9.0 {
                        let from = self.players[id].p;
                        let carrier = self.players[o].p;
                        let to = clamp_pitch(carrier + self.players[o].facing * 1.1, 0.0);
                        self.players[id].p = to;
                        self.players[id].facing = (carrier - to).normalize_or(Vec2::X);
                        self.events.push(SimEvent::Teleport { player: id, from, to });
                        let tdir = (self.ball.xy() - to).normalize_or(Vec2::X);
                        self.players[id].action = Action::Tackle { t: 0.0, dir: tdir, flux: true, resolved: false };
                    } else {
                        self.players[id].flux = None;
                        self.events.push(SimEvent::FluxFizzle { player: id });
                    }
                }
            }
            FluxAction::SuperJump => {
                self.players[id].action = Action::Jump { t: 0.0 };
                let d = self.dist_to_ball(id);
                let z = self.ball.p.z;
                if self.ball.owner.is_none() && d < 2.5 && z > 0.9 && z < 4.5 {
                    let dir_sign = self.attack_dir(team);
                    let goal = Vec2::new(goal_x(dir_sign), 0.0);
                    let to = (goal - self.players[id].p).normalize_or(Vec2::new(dir_sign, 0.0));
                    let speed = 16.0 + 8.0 * n(self.players[id].stats.heading);
                    let b = &mut self.ball;
                    b.v = Vec3::new(to.x * speed, to.y * speed, -1.5);
                    b.p.z = b.p.z.min(2.2);
                    b.owner = None;
                    b.kicked_by = Some(id);
                    b.since_kick = 0.0;
                    b.last_touch = Some(id);
                    b.last_team = Some(team);
                    b.flags = Default::default();
                    b.flags.flux_shot = true;
                    self.events.push(SimEvent::Kick { player: id, power: 0.8, kind: KickKind::Shot, flux: true });
                }
            }
            FluxAction::EclipseCloud => {
                let at = self.players[id].p;
                self.clouds.push(Cloud { p: at, r: 5.0, team, t: 3.0 });
                self.events.push(SimEvent::Cloud { team, at, radius: 5.0 });
            }
        }
    }

    /// If an opponent nearby started a Flux action moments ago, resolve a
    /// duel. Returns true when a duel happened.
    fn check_duel(&mut self, id: usize) -> bool {
        let team = self.players[id].team;
        let now = self.time;
        let me_p = self.players[id].p;
        let mut found = None;
        for &o in &self.teams[Self::opponent(team)].players {
            if let Some(f) = self.players[o].flux
                && now - f.started_at <= flux::DUEL_WINDOW && now - f.started_at > 0.0 && self.players[o].p.distance(me_p) < flux::DUEL_RANGE {
                    found = Some(o);
                    break;
                }
        }
        let Some(o) = found else { return false };
        self.duel(id, o, 0.1);
        true
    }

    /// `bonus` goes to the attacker (the one reacting second, or the shooter).
    fn duel(&mut self, attacker: usize, defender: usize, bonus: f32) {
        let a = n(self.players[attacker].stats.flux_power) * (0.8 + 0.4 * self.rng.f32()) + bonus;
        let d = n(self.players[defender].stats.flux_power) * (0.8 + 0.4 * self.rng.f32());
        let a = if self.players[attacker].fatigued() { a * 0.7 } else { a };
        let d = if self.players[defender].fatigued() { d * 0.7 } else { d };
        let winner = if a >= d { attacker } else { defender };
        let loser = if winner == attacker { defender } else { attacker };
        self.players[loser].flux = None;
        self.players[loser].flux_cooldown = 1.0;
        self.players[loser].strain = (self.players[loser].strain + 15.0).min(100.0);
        if let Action::WindUp { .. } = self.players[loser].action {
            self.players[loser].action = Action::Stunned { t: 0.2 };
        }
        self.events.push(SimEvent::Duel { attacker, defender, winner });
        self.events.push(SimEvent::FluxFizzle { player: loser });
        self.slowmo = 40;
    }

    // ------------------------------------------------------------------ ball

    fn update_ball(&mut self, dt: f32) {
        let now = self.time;
        if let Some(o) = self.ball.owner {
            let (p, v, facing, team) = {
                let pl = &self.players[o];
                (pl.p, pl.v, pl.facing, pl.team)
            };
            let sprinting = v.length() > 6.5;
            let gap = 0.45 + if sprinting { 0.6 } else { 0.0 };
            self.ball.p = Vec3::new(p.x + facing.x * gap, p.y + facing.y * gap, BALL_RADIUS);
            self.ball.prev = self.ball.p;
            self.ball.v = Vec3::new(v.x, v.y, 0.0);
            self.ball.since_kick += dt;
            // Contact steals: an opponent on the ball has a small chance per tick.
            if !self.is_dead_ball() {
                let bxy = self.ball.xy();
                let mut thief = None;
                for &j in &self.teams[Self::opponent(team)].players {
                    let pj = &self.players[j];
                    if pj.action.can_act() && pj.p.distance(bxy) < 0.75 && pj.v.length() > 1.0 {
                        let carrier = &self.players[o];
                        let p = 0.014 * (0.6 + n(pj.stats.tackle) + n(pj.stats.strength) * 0.5 - n(carrier.stats.ball_control) * 0.8 - n(carrier.stats.strength) * 0.3).max(0.1);
                        if self.rng.chance(p) {
                            thief = Some(j);
                            break;
                        }
                    }
                }
                if let Some(j) = thief {
                    let jf = self.players[j].facing;
                    let b = &mut self.ball;
                    b.owner = None;
                    b.v = Vec3::new(jf.x * 2.5, jf.y * 2.5, 0.2);
                    b.kicked_by = Some(j);
                    b.since_kick = 0.25;
                    b.last_touch = Some(j);
                    b.last_team = Some(self.players[j].team);
                    self.events.push(SimEvent::Tackle { by: j, on: o, success: true });
                    let jt = self.players[j].team;
                    self.gain(jt, PoolGain::TackleWon);
                }
            }
            return;
        }

        let blizzard = (0..2).any(|t| self.teams[t].team_flux_active(now) && self.teams[t].kind == FluxKind::Breath);
        let friction = if self.ball.flags.ice_lane { 0.25 } else if blizzard { 0.5 } else { 1.0 };
        if let Some(speed) = self.ball.integrate(dt, friction)
            && speed > 1.5 {
                self.events.push(SimEvent::Bounce { pos: self.ball.xy(), speed });
            }
        if self.ball.v.length() < 0.2 && self.ball.p.z <= BALL_RADIUS + 0.01 {
            self.ball.flags.ice_lane = false;
        }

        if self.is_dead_ball() {
            return;
        }

        // Possession and headers, closest player first.
        let bxy = self.ball.xy();
        let bz = self.ball.p.z;
        let mut order: Vec<usize> = (0..self.players.len()).collect();
        order.sort_by(|&a, &b| {
            self.players[a].p.distance(bxy).partial_cmp(&self.players[b].p.distance(bxy)).unwrap_or(std::cmp::Ordering::Equal)
        });
        for id in order {
            let pl = &self.players[id];
            let team = pl.team;
            let d = pl.p.distance(bxy);
            let reach = 0.75 + 0.3 * n(pl.stats.ball_control);
            if d > 1.0 {
                break;
            }
            if !(pl.action.can_act() || matches!(pl.action, Action::Kick { t } if t > 0.15) || matches!(pl.action, Action::Jump { .. })) {
                continue;
            }
            if self.ball.kicked_by == Some(id) && self.ball.since_kick < 0.35 {
                continue;
            }
            let opponent_ball = self.ball.last_team.is_some() && self.ball.last_team != Some(team);
            if opponent_ball {
                if self.ball.flags.ice_lane && pl.flux.is_none() {
                    continue;
                }
                if self.ball_hidden() {
                    continue;
                }
                if self.ball.flags.phase {
                    self.ball.flags.phase = false;
                    continue;
                }
            }
            let max_z = if self.ball.flags.high_breath && !opponent_ball { 3.6 } else { 1.15 };
            if d <= reach && bz <= max_z {
                let incoming = Vec2::new(self.ball.v.x, self.ball.v.y).length();
                let control = n(pl.stats.ball_control);
                if incoming > 13.0 + 10.0 * control && self.rng.chance(0.5 - control * 0.4) {
                    // Heavy touch: the ball bounces off.
                    let b = &mut self.ball;
                    let away = (bxy - pl.p).normalize_or(pl.facing);
                    b.v = Vec3::new(away.x * incoming * 0.3, away.y * incoming * 0.3, 1.0);
                    b.kicked_by = Some(id);
                    b.since_kick = 0.0;
                    b.last_touch = Some(id);
                    b.last_team = Some(team);
                    break;
                }
                let b = &mut self.ball;
                b.owner = Some(id);
                b.v = Vec3::ZERO;
                b.spin = 0.0;
                b.p.z = BALL_RADIUS;
                b.flags = Default::default();
                b.last_touch = Some(id);
                let prev_team = b.last_team;
                b.last_team = Some(team);
                self.events.push(SimEvent::Possession { player: id });
                if let Some((pt, passer, _)) = self.pending_pass.take()
                    && pt == team && passer != id {
                        self.gain(team, PoolGain::PassComplete);
                    }
                if prev_team.is_some() && prev_team != Some(team) {
                    self.gain(team, PoolGain::Interception);
                }
                if self.teams[team].human && !self.players[id].is_gk() {
                    self.teams[team].controlled = Some(id);
                    self.teams[team].last_switch = now;
                }
                self.teams[team].gk_hold = 0.0;
                break;
            } else if d <= 0.9 && bz > 1.15 && bz < 2.4 && self.ball.v.z < 1.0 {
                // Header: toward goal if attacking, clear if defending.
                let dir_sign = self.attack_dir(team);
                let gx = goal_x(dir_sign);
                let attacking = (gx - pl.p.x) * dir_sign < 22.0;
                let to = if attacking {
                    (Vec2::new(gx, 0.0) - pl.p).normalize_or(Vec2::new(dir_sign, 0.0))
                } else {
                    Vec2::new(dir_sign, pl.p.y.signum() * 0.6).normalize()
                };
                let speed = 6.0 + 9.0 * n(pl.stats.heading);
                let b = &mut self.ball;
                b.v = Vec3::new(to.x * speed, to.y * speed, if attacking { -1.0 } else { 2.5 });
                b.kicked_by = Some(id);
                b.since_kick = 0.0;
                b.last_touch = Some(id);
                b.last_team = Some(team);
                b.flags = Default::default();
                self.players[id].action = Action::Jump { t: 0.0 };
                self.events.push(SimEvent::Kick { player: id, power: 0.5, kind: KickKind::Lob, flux: false });
                break;
            }
        }
    }

    // --------------------------------------------------------------- keepers

    fn update_keepers(&mut self, dt: f32) {
        let now = self.time;
        for team in 0..2 {
            let gk = self.keeper(team);
            let own_dir = -self.attack_dir(team);
            let gx = goal_x(own_dir);
            let opp = Self::opponent(team);

            if self.ball.owner == Some(gk) {
                self.teams[team].gk_hold += dt;
                self.teams[team].gk_shot_seen = None;
                if self.teams[team].gk_hold > 1.0 && !self.is_dead_ball() {
                    // Distribute: short to an unmarked defender, else long.
                    let mut best = None;
                    let mut bs = f32::MIN;
                    for &t in &self.teams[team].players[1..] {
                        let tp = self.players[t].p;
                        let mut near = f32::MAX;
                        for &o in &self.teams[opp].players {
                            near = near.min(self.players[o].p.distance(tp));
                        }
                        let s = near.min(8.0) - tp.distance(self.players[gk].p) * 0.1;
                        if s > bs {
                            bs = s;
                            best = Some(t);
                        }
                    }
                    let kind = if bs > 4.0 { KickKind::Pass } else { KickKind::Lob };
                    let dir = best.map(|t| (self.players[t].p - self.players[gk].p).normalize_or(Vec2::X)).unwrap_or(Vec2::new(-own_dir, 0.0));
                    self.execute_kick(gk, KickCmd { kind, power: if kind == KickKind::Lob { 0.9 } else { 0.5 }, dir, target: best, flux: false, manual: false }, None);
                    self.teams[team].gk_hold = 0.0;
                }
                continue;
            }
            if !matches!(self.phase, Phase::Play) {
                continue;
            }

            // Shot detection.
            let b = &self.ball;
            let toward = b.owner.is_none() && b.v.x * own_dir > 5.0 && b.last_team == Some(opp);
            let mut shot_target = None;
            if toward {
                let t_line = (gx - b.p.x) / b.v.x;
                if t_line > 0.0 && t_line < 2.5 {
                    let py = b.p.y + b.v.y * t_line;
                    let pz = (b.p.z + b.v.z * t_line - 0.5 * 9.81 * t_line * t_line).max(0.0);
                    if py.abs() < GOAL_HALF_WIDTH + 1.2 && pz < GOAL_HEIGHT + 1.2 {
                        shot_target = Some((Vec2::new(gx - own_dir * 0.5, py.clamp(-(GOAL_HALF_WIDTH + 0.8), GOAL_HALF_WIDTH + 0.8)), t_line));
                    }
                }
            }
            if let Some((target, _t_line)) = shot_target {
                if self.teams[team].gk_shot_seen.is_none() {
                    self.teams[team].gk_shot_seen = Some(now);
                    self.teams[team].gk_dive_target = Some(target);
                    // Keeper Flux.
                    let want = self.teams[team].pool >= CHARGE && self.players[gk].can_use_flux();
                    let willing = if self.teams[team].human { 0.85 } else { 0.5 + 0.4 * self.cfg.difficulty };
                    if want && self.rng.chance(willing) {
                        let fa = flux::map(self.teams[team].kind, BaseAction::Keeper);
                        if self.try_start_flux(gk, fa, Vec2::ZERO, None) && fa == FluxAction::ShadowKeeper {
                            let from = self.players[gk].p;
                            self.players[gk].p = target;
                            self.events.push(SimEvent::Teleport { player: gk, from, to: target });
                        }
                    }
                    if self.ball.flags.flux_shot
                        && let Some(shooter) = self.ball.kicked_by
                            && self.players[gk].flux.is_some() {
                                self.duel(shooter, gk, 0.05);
                                if self.players[gk].flux.is_some() {
                                    // Keeper won: the shot loses its edge.
                                    self.ball.flags.flux_shot = false;
                                    self.ball.v *= 0.7;
                                }
                            }
                }
                let seen = self.teams[team].gk_shot_seen.unwrap_or(now);
                let reflex = n(self.players[gk].stats.gk_reflex);
                let mut reaction = 0.42 - 0.3 * reflex;
                if self.ball_hidden() {
                    reaction += 0.3;
                }
                if self.players[gk].flux.is_none() && self.ball.flags.flux_shot {
                    reaction += 0.08;
                }
                if now - seen >= reaction && !matches!(self.players[gk].action, Action::Stunned { .. }) {
                    let target = self.teams[team].gk_dive_target.unwrap_or(target);
                    let reach_n = n(self.players[gk].stats.gk_reach);
                    let speed = 7.0 + 5.0 * reach_n;
                    let d = target - self.players[gk].p;
                    if d.length() > 0.2 {
                        let v = d.normalize() * speed;
                        self.players[gk].v = v;
                        self.players[gk].p += v * dt;
                    }
                    self.players[gk].action = Action::Dive { t: 0.0 };
                    self.players[gk].facing = Vec2::new(-own_dir, 0.0);
                }
            } else {
                self.teams[team].gk_shot_seen = None;
                self.teams[team].gk_dive_target = None;
            }

            // Save attempt.
            let reach_n = n(self.players[gk].stats.gk_reach);
            let mut reach = 0.95 + 0.6 * reach_n;
            if self.players[gk].flux_active(FluxAction::WallOfIce) {
                reach *= 1.6;
            }
            let bxy = self.ball.xy();
            let b_in = self.ball.owner.is_none() && self.ball.last_team == Some(opp);
            let moving_in = self.ball.v.x * own_dir > 1.0;
            if b_in && moving_in && self.players[gk].p.distance(bxy) < reach && self.ball.p.z < 2.6 && !matches!(self.players[gk].action, Action::Stunned { .. }) {
                let speed = self.ball.v.length();
                let reflex = n(self.players[gk].stats.gk_reflex);
                let catch_limit = 13.0 + 11.0 * reflex - if self.ball.flags.flux_shot { 7.0 } else { 0.0 };
                if speed < catch_limit {
                    let gp = self.players[gk].p;
                    let b = &mut self.ball;
                    b.owner = Some(gk);
                    b.v = Vec3::ZERO;
                    b.flags = Default::default();
                    b.last_touch = Some(gk);
                    b.last_team = Some(team);
                    b.p = Vec3::new(gp.x - own_dir * 0.5, gp.y, BALL_RADIUS);
                    self.teams[team].gk_hold = 0.0;
                    self.events.push(SimEvent::Save { keeper: gk, caught: true });
                } else {
                    let side = self.rng.sym();
                    let gpx = self.players[gk].p.x;
                    let b = &mut self.ball;
                    b.v = Vec3::new(-own_dir * speed * 0.35, side * speed * 0.45, 2.5);
                    b.p.x = gpx - own_dir * 0.6;
                    b.kicked_by = Some(gk);
                    b.since_kick = 0.0;
                    b.last_touch = Some(gk);
                    b.last_team = Some(team);
                    let flux_shot = b.flags.flux_shot;
                    b.flags = Default::default();
                    if flux_shot {
                        self.players[gk].action = Action::Stunned { t: 0.0 };
                    }
                    self.events.push(SimEvent::Save { keeper: gk, caught: false });
                }
                self.gain(opp, PoolGain::ShotOnTarget);
                self.teams[team].gk_shot_seen = None;
                self.players[gk].flux = None;
            }
        }
    }

    // ----------------------------------------------------------------- rules

    fn rules(&mut self) {
        if !matches!(self.phase, Phase::Play) {
            return;
        }
        let b = &self.ball;
        // Goal?
        for dir in [-1.0f32, 1.0] {
            let crossed = b.prev.x * dir < HALF_LEN && b.p.x * dir >= HALF_LEN;
            if crossed {
                let t = ((HALF_LEN * dir - b.prev.x) / (b.p.x - b.prev.x)).clamp(0.0, 1.0);
                let hit = b.prev.lerp(b.p, t);
                if hit.y.abs() < GOAL_HALF_WIDTH && hit.z < GOAL_HEIGHT {
                    let team = if self.attack_dir(0) == dir { 0 } else { 1 };
                    let scorer = b.last_touch.unwrap_or(self.teams[team].players[6]);
                    self.teams[team].score += 1;
                    self.events.push(SimEvent::Goal { team, scorer });
                    self.phase = Phase::Goal { team, t: 0.0 };
                    self.gain(Self::opponent(team), PoolGain::Conceded);
                    self.ball.owner = None;
                    return;
                }
            }
        }
        let b = &self.ball;
        let last_team = b.last_team.unwrap_or(0);
        if b.p.x.abs() > HALF_LEN + 0.4 {
            let inside_goal = b.p.y.abs() < GOAL_HALF_WIDTH + 0.3 && b.p.z < GOAL_HEIGHT + 0.3 && b.p.x.abs() < HALF_LEN + GOAL_DEPTH;
            if inside_goal {
                return;
            }
            let sign = b.p.x.signum();
            let attacker = if self.attack_dir(0) == sign { 0 } else { 1 };
            let defender = Self::opponent(attacker);
            if last_team == defender {
                let at = Vec2::new(sign * (HALF_LEN - 0.5), b.p.y.signum() * (HALF_WID - 0.5));
                self.set_dead_ball(Restart::Corner, attacker, at);
            } else {
                let at = Vec2::new(sign * (HALF_LEN - 5.0), b.p.y.signum() * 3.0);
                self.set_dead_ball(Restart::GoalKick, defender, at);
            }
            return;
        }
        if b.p.y.abs() > HALF_WID + 0.4 {
            let at = Vec2::new(b.p.x.clamp(-HALF_LEN + 1.0, HALF_LEN - 1.0), b.p.y.signum() * (HALF_WID - 0.3));
            self.set_dead_ball(Restart::ReEntry, Self::opponent(last_team), at);
        }
    }

    fn update_switching(&mut self, human: &[Option<InputFrame>; 2]) {
        let now = self.time;
        for team in 0..2 {
            if !self.teams[team].human {
                continue;
            }
            let Some(inp) = human[team] else { continue };
            let bxy = self.ball.xy();
            let cur = self.teams[team].controlled;
            let mut ranked: Vec<usize> = self.teams[team].players[1..].to_vec();
            ranked.sort_by(|&a, &b| {
                self.players[a].p.distance(bxy).partial_cmp(&self.players[b].p.distance(bxy)).unwrap_or(std::cmp::Ordering::Equal)
            });
            if inp.switch.pressed {
                let next = ranked.iter().copied().find(|&id| Some(id) != cur).unwrap_or(ranked[0]);
                self.teams[team].controlled = Some(next);
                self.teams[team].last_switch = now;
                continue;
            }
            let own_ball = matches!(self.ball.owner, Some(o) if self.players[o].team == team)
                || (self.ball.owner.is_none() && self.ball.last_team == Some(team) && self.ball.since_kick < 1.5);
            if own_ball || self.is_dead_ball() {
                continue;
            }
            if now - self.teams[team].last_switch < 0.6 {
                continue;
            }
            let closest = ranked[0];
            if Some(closest) != cur {
                let cd = cur.map(|c| self.players[c].p.distance(bxy)).unwrap_or(f32::MAX);
                if self.players[closest].p.distance(bxy) + 2.5 < cd {
                    self.teams[team].controlled = Some(closest);
                    self.teams[team].last_switch = now;
                }
            }
        }
    }
}

impl Pos {
    pub fn label(self) -> &'static str {
        match self {
            Pos::GK => "GK",
            Pos::DF => "DF",
            Pos::MF => "MF",
            Pos::ST => "ST",
        }
    }
}
