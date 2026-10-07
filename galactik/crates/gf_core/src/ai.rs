//! Team AI. Produces `InputFrame`s, exactly like a human controller would,
//! so the simulation has a single code path.

use crate::ball::Ball;
use crate::data::{FluxKind, Pos};
use crate::flux::{CHARGE, POOL_MAX};
use crate::input::{Button, InputFrame, KickCmd, KickKind};
use crate::pitch::*;
use crate::rng::Rng;
use crate::sim::{Phase, Sim, DT};
use glam::Vec2;

fn n(v: u8) -> f32 {
    v as f32 / 99.0
}

fn pressed() -> Button {
    Button { held: true, pressed: true, released: false }
}

/// Where the ball will be, sampled every 6 ticks for up to 2.5 s.
fn predict_ball(sim: &Sim) -> Vec<(f32, Vec2)> {
    let mut b: Ball = sim.ball.clone();
    let mut out = Vec::with_capacity(26);
    let mut t = 0.0;
    out.push((0.0, b.xy()));
    for i in 1..=150 {
        b.integrate(DT, 1.0);
        t += DT;
        if i % 6 == 0 {
            out.push((t, b.xy()));
            if b.v.length() < 0.3 {
                break;
            }
        }
    }
    out
}

fn intercept_point(sim: &Sim, p: Vec2, speed: f32) -> Vec2 {
    if let Some(o) = sim.ball.owner {
        let c = &sim.players[o];
        return c.p + c.v * 0.3;
    }
    let path = predict_ball(sim);
    for (t, bp) in &path {
        if bp.distance(p) / speed <= *t + 0.05 {
            return clamp(*bp);
        }
    }
    clamp(path.last().map(|x| x.1).unwrap_or(sim.ball.xy()))
}

fn clamp(p: Vec2) -> Vec2 {
    Vec2::new(p.x.clamp(-HALF_LEN + 0.5, HALF_LEN - 0.5), p.y.clamp(-HALF_WID + 0.5, HALF_WID - 0.5))
}

fn team_has_ball(sim: &Sim, team: usize) -> bool {
    match sim.ball.owner {
        Some(o) => sim.players[o].team == team,
        None => sim.ball.last_team == Some(team) && sim.ball.since_kick < 2.0,
    }
}

fn can_see_ball(sim: &Sim, team: usize) -> bool {
    if sim.ball.last_team == Some(team) {
        return true;
    }
    if sim.ball_hidden() {
        return false;
    }
    !sim.in_cloud(sim.ball.xy(), team)
}

/// Rank of `id` among its outfield teammates by distance to a point (0 = closest).
fn rank_to(sim: &Sim, id: usize, point: Vec2) -> usize {
    let team = sim.players[id].team;
    let my = sim.players[id].p.distance(point);
    sim.teams[team].players[1..]
        .iter()
        .filter(|&&o| o != id && sim.players[o].p.distance(point) < my)
        .count()
}

fn nearest_opponent(sim: &Sim, team: usize, p: Vec2) -> (Option<usize>, f32) {
    let mut best = None;
    let mut bd = f32::MAX;
    for &o in &sim.teams[Sim::opponent(team)].players {
        let d = sim.players[o].p.distance(p);
        if d < bd {
            bd = d;
            best = Some(o);
        }
    }
    (best, bd)
}

/// Where this player wants to be, and whether to sprint there.
fn target_point(sim: &Sim, id: usize) -> (Vec2, bool) {
    let me = &sim.players[id];
    let team = me.team;
    let dir = sim.attack_dir(team);
    let own_dir = -dir;
    let slot = sim.slot_of(id);
    let ball = sim.ball.xy();
    let sees = can_see_ball(sim, team);
    let speed = 5.2 + 3.2 * n(me.stats.pace);

    if me.is_gk() {
        let gx = goal_x(own_dir);
        let base = Vec2::new(gx - own_dir * 1.8, (ball.y * 0.35).clamp(-2.8, 2.8));
        let free = sim.ball.owner.is_none();
        if free && sees && in_box(ball, own_dir) && me.p.distance(ball) < 7.0 {
            let (_, od) = nearest_opponent(sim, team, ball);
            if od > me.p.distance(ball) + 0.5 {
                return (intercept_point(sim, me.p, speed * 1.3), true);
            }
        }
        return (base, false);
    }

    // Dead ball: the taker walks to the ball (handled by the sim); others hold.
    if sim.is_dead_ball() {
        let attack = matches!(sim.phase, Phase::DeadBall { team: t, .. } if t == team);
        let a = sim.anchor_world(team, attack, slot);
        return (a, false);
    }

    let has = team_has_ball(sim, team);
    let free = sim.ball.owner.is_none();

    if free && sees {
        let rank = rank_to(sim, id, ball);
        if rank == 0 || (rank == 1 && me.p.distance(ball) < 6.0) {
            return (intercept_point(sim, me.p, speed * 1.3), true);
        }
    }

    if has {
        let mut a = sim.anchor_world(team, true, slot);
        a += Vec2::new(ball.x * 0.35, ball.y * 0.3);
        if me.pos == Pos::ST && ball.x * dir > a.x * dir - 6.0 {
            a.x += dir * 5.0;
        }
        if let Some(o) = sim.ball.owner {
            let c = sim.players[o].p;
            if c.distance(a) < 5.0 {
                let away = (a - c).normalize_or(Vec2::new(0.0, 1.0));
                a = c + away * 5.5;
            }
        }
        let a = clamp(a);
        let far = me.p.distance(a) > 7.0;
        return (a, far);
    }

    // Defending.
    let carrier_p = sim.ball.owner.map(|o| sim.players[o].p + sim.players[o].v * 0.3).unwrap_or(ball);
    if sees {
        let rank = rank_to(sim, id, carrier_p);
        if rank == 0 {
            return (carrier_p, true);
        }
        if rank == 1 {
            let goal = Vec2::new(goal_x(own_dir), 0.0);
            let cover = carrier_p + (goal - carrier_p).normalize_or(Vec2::X) * 4.0;
            return (clamp(cover), true);
        }
    }
    let mut a = sim.anchor_world(team, false, slot);
    if sees {
        a += Vec2::new(ball.x * 0.4, ball.y * 0.35);
        // Zonal marking: pull toward the nearest opponent near the anchor.
        let mut best = None;
        let mut bd = 7.0;
        for &o in &sim.teams[Sim::opponent(team)].players {
            let d = sim.players[o].p.distance(a);
            if d < bd && !sim.players[o].is_gk() {
                bd = d;
                best = Some(o);
            }
        }
        if let Some(o) = best {
            let op = sim.players[o].p;
            let goal = Vec2::new(goal_x(own_dir), 0.0);
            let mark = op + (goal - op).normalize_or(Vec2::X) * 1.5;
            a = a.lerp(mark, 0.6);
        }
    }
    let a = clamp(a);
    (a, me.p.distance(a) > 8.0)
}

pub fn movement_only(sim: &Sim, id: usize, prev: Vec2) -> Vec2 {
    if sim.ball.owner == Some(id) {
        return prev;
    }
    let (t, _) = target_point(sim, id);
    let d = t - sim.players[id].p;
    if d.length() < 0.4 {
        Vec2::ZERO
    } else {
        d.normalize()
    }
}

pub fn decide(sim: &Sim, rng: &mut Rng, id: usize) -> (InputFrame, f32) {
    let me = &sim.players[id];
    let team = me.team;
    let diff = if sim.teams[team].human { 0.7 } else { sim.cfg.difficulty };
    let mut f = InputFrame::default();
    let hold = 0.22 + (1.0 - diff) * 0.25;
    let now = sim.time;

    // Team Flux when the pool is full and the moment is right.
    let ts = &sim.teams[team];
    if ts.pool >= POOL_MAX - 0.5 && !ts.team_flux_active(now) {
        let behind = ts.score < sim.teams[Sim::opponent(team)].score;
        let late = sim.clock > sim.cfg.half_len_secs * 0.55;
        if (behind || late || rng.chance(0.15)) && rng.chance(0.4) {
            f.team_flux = pressed();
        }
    }

    // Dead ball taker.
    if sim.is_dead_ball() {
        if sim.dead_ball_taker() == Some(id) {
            let t = match sim.phase {
                Phase::KickOff { t, .. } | Phase::DeadBall { t, .. } => t,
                _ => 0.0,
            };
            if t > 0.9 && sim.ball.owner == Some(id) {
                let restart = match sim.phase {
                    Phase::DeadBall { restart, .. } => Some(restart),
                    _ => None,
                };
                let dir = sim.attack_dir(team);
                let gx = goal_x(dir);
                let kick = match restart {
                    Some(crate::events::Restart::Penalty) => KickCmd {
                        kind: KickKind::Shot,
                        power: 0.8,
                        dir: Vec2::new(0.0, rng.sym().signum() * 0.9),
                        target: None,
                        flux: false,
                        manual: false,
                    },
                    Some(crate::events::Restart::Corner) => {
                        let target = sim.teams[team].players[6];
                        KickCmd { kind: KickKind::Lob, power: 0.6, dir: (sim.players[target].p - me.p).normalize_or(Vec2::X), target: Some(target), flux: false, manual: false }
                    }
                    Some(crate::events::Restart::FreeKick) if (gx - me.p.x).abs() < 22.0 => KickCmd {
                        kind: KickKind::Shot,
                        power: 0.85,
                        dir: Vec2::new(0.0, rng.sym() * 0.7),
                        target: None,
                        flux: sim.teams[team].pool >= CHARGE,
                        manual: false,
                    },
                    _ => {
                        let aim = Vec2::new(dir, 0.0);
                        let target = sim.pick_pass_target(id, aim, KickKind::Pass).or_else(|| sim.pick_pass_target(id, -aim, KickKind::Pass));
                        let kind = if target.map(|t| sim.players[t].p.distance(me.p) > 18.0).unwrap_or(true) { KickKind::Lob } else { KickKind::Pass };
                        KickCmd { kind, power: 0.6, dir: aim, target, flux: false, manual: false }
                    }
                };
                f.kick = Some(kick);
            }
            return (f, 0.1);
        }
        let (t, _) = target_point(sim, id);
        f.movement = (t - me.p).normalize_or(Vec2::ZERO) * if me.p.distance(t) > 0.5 { 1.0 } else { 0.0 };
        return (f, 0.3);
    }

    // Carrier.
    if sim.ball.owner == Some(id) {
        return (decide_carrier(sim, rng, id), hold);
    }

    let (target, sprint) = target_point(sim, id);
    let d = target - me.p;
    f.movement = if d.length() > 0.4 { d.normalize() } else { Vec2::ZERO };
    f.sprint = Button { held: sprint, pressed: false, released: false };

    // Defending: tackle when on the carrier.
    if let Some(o) = sim.ball.owner {
        if sim.players[o].team != team && !me.is_gk() {
            let dist = me.p.distance(sim.ball.xy());
            let pool = sim.teams[team].pool;
            let kind = sim.teams[team].kind;
            let flux_ok = pool >= CHARGE && me.can_use_flux() && !me.fatigued();
            if dist < 1.5 && rng.chance(0.55 + diff * 0.3) {
                f.action_a = pressed();
                if flux_ok && kind == FluxKind::Breath && rng.chance(0.5 * diff) {
                    f.flux = true;
                }
            } else if dist < 7.0 && dist > 2.0 && flux_ok && kind == FluxKind::Smog && rng.chance(0.25 * diff) && rank_to(sim, id, sim.ball.xy()) == 0 {
                f.flux = true;
                f.action_a = pressed();
            } else if dist < 3.0 && dist > 1.5 && rng.chance(0.12) && me.p.x * sim.attack_dir(team) > sim.ball.xy().x * sim.attack_dir(team) {
                f.slide = pressed();
            }
            // Smog: an Eclipse Cloud when the carrier is in our half.
            if kind == FluxKind::Smog && flux_ok && dist < 6.0 && sim.ball.xy().x * sim.attack_dir(team) < 0.0 && rng.chance(0.08 * diff) {
                f.special = pressed();
            }
        }
    }
    // Breath: super-jump onto a high ball near us.
    if sim.teams[team].kind == FluxKind::Breath && sim.ball.owner.is_none() {
        let z = sim.ball.p.z;
        if z > 1.3 && z < 4.0 && me.p.distance(sim.ball.xy()) < 2.2 && sim.teams[team].pool >= CHARGE && me.can_use_flux() && rng.chance(0.5) {
            let gx = goal_x(sim.attack_dir(team));
            if (gx - me.p.x).abs() < 25.0 {
                f.special = pressed();
            }
        }
    }
    (f, if sprint { 0.35 } else { 0.45 })
}

fn decide_carrier(sim: &Sim, rng: &mut Rng, id: usize) -> InputFrame {
    let me = &sim.players[id];
    let team = me.team;
    let diff = if sim.teams[team].human { 0.7 } else { sim.cfg.difficulty };
    let dir = sim.attack_dir(team);
    let gx = goal_x(dir);
    let goal = Vec2::new(gx, 0.0);
    let to_goal = goal - me.p;
    let goal_dist = to_goal.length();
    let pool = sim.teams[team].pool;
    let flux_ok = pool >= CHARGE && me.can_use_flux() && !me.fatigued();
    let (near_opp, near_d) = nearest_opponent(sim, team, me.p);
    let pressed_hard = near_d < 2.5;
    let mut f = InputFrame::default();

    // Keeper with the ball is handled by the sim.
    if me.is_gk() {
        return f;
    }

    // Shot?
    let open_lane = {
        let dn = to_goal / goal_dist.max(0.1);
        let mut blocked = 0;
        for &o in &sim.teams[Sim::opponent(team)].players {
            let op = sim.players[o].p;
            if sim.players[o].is_gk() {
                continue;
            }
            let t = ((op - me.p).dot(dn) / goal_dist).clamp(0.0, 1.0);
            let c = me.p + dn * (t * goal_dist);
            if t > 0.05 && c.distance(op) < 1.2 {
                blocked += 1;
            }
        }
        blocked == 0
    };
    let angle_ok = (me.p.y.abs() / (gx - me.p.x).abs().max(1.0)) < 1.1;
    let shoot_range = 14.0 + 12.0 * n(me.stats.shooting);
    if goal_dist < shoot_range && angle_ok && (open_lane || goal_dist < 10.0) && rng.chance(0.45 + diff * 0.4) {
        let flux = flux_ok && rng.chance(0.35 + diff * 0.4);
        f.kick = Some(KickCmd {
            kind: KickKind::Shot,
            power: (0.55 + goal_dist / 40.0 + rng.f32() * 0.2).min(1.0),
            dir: Vec2::new(0.0, rng.sym() * 0.8),
            target: None,
            flux,
            manual: false,
        });
        return f;
    }

    // Pass options.
    let mut best: Option<(usize, f32, KickKind)> = None;
    for &t in &sim.teams[team].players[1..] {
        if t == id {
            continue;
        }
        let o = &sim.players[t];
        let d = o.p - me.p;
        let dist = d.length();
        if dist < 2.0 || dist > 32.0 {
            continue;
        }
        let dn = d / dist;
        let (_, od) = nearest_opponent(sim, team, o.p);
        let mut blocked = 0.0;
        for &x in &sim.teams[Sim::opponent(team)].players {
            let xp = sim.players[x].p;
            let tt = ((xp - me.p).dot(dn) / dist).clamp(0.0, 1.0);
            let c = me.p + dn * (tt * dist);
            if tt > 0.05 && tt < 0.95 && c.distance(xp) < 1.4 {
                blocked += 1.0;
            }
        }
        let progress = d.x * dir / 30.0;
        let space_ahead = {
            let ahead = o.p + Vec2::new(dir * 6.0, 0.0);
            nearest_opponent(sim, team, ahead).1
        };
        let through = space_ahead > 4.0 && d.x * dir > 4.0 && o.pos != Pos::DF;
        let score = od.min(7.0) / 7.0 * 1.2 + progress * 0.8 - blocked * 1.3 - dist / 60.0 + if through { 0.3 } else { 0.0 };
        if best.map(|b| score > b.1).unwrap_or(true) {
            best = Some((t, score, if through { KickKind::Through } else { KickKind::Pass }));
        }
    }
    let (_, my_space) = nearest_opponent(sim, team, me.p + to_goal.normalize_or(Vec2::X) * 4.0);
    let want_pass = match best {
        Some((_, s, _)) => (pressed_hard && s > -0.2) || s > 0.9 || (my_space < 3.0 && s > 0.4),
        None => false,
    };
    let near_byline = (gx - me.p.x).abs() < 8.0 && me.p.y.abs() > 10.0;
    if near_byline {
        let st = sim.teams[team].players[6];
        let st = if st == id { sim.teams[team].players[5] } else { st };
        f.kick = Some(KickCmd { kind: KickKind::Lob, power: 0.6, dir: (sim.players[st].p - me.p).normalize_or(Vec2::X), target: Some(st), flux: false, manual: false });
        return f;
    }
    if want_pass && rng.chance(0.5 + diff * 0.45) {
        let (t, _, kind) = best.unwrap();
        let dist = sim.players[t].p.distance(me.p);
        let kind = if dist > 24.0 { KickKind::Lob } else { kind };
        let flux = flux_ok && kind != KickKind::Lob && pressed_hard && rng.chance(0.3 * diff);
        f.kick = Some(KickCmd {
            kind,
            power: (dist / 30.0 + 0.3).clamp(0.3, 1.0),
            dir: (sim.players[t].p - me.p).normalize_or(Vec2::X),
            target: Some(t),
            flux,
            manual: false,
        });
        return f;
    }

    // Dribble: toward goal, steering around the nearest opponent.
    let mut dirv = to_goal.normalize_or(Vec2::new(dir, 0.0));
    if let Some(o) = near_opp {
        let op = sim.players[o].p;
        let rel = op - me.p;
        if rel.length() < 6.0 && rel.dot(dirv) > 0.0 {
            let side = me.ai_dribble_side;
            let perp = Vec2::new(-dirv.y, dirv.x) * side;
            dirv = (dirv + perp * (1.0 - rel.length() / 6.0) * 1.5).normalize_or(dirv);
        }
    }
    // Stay off the touchline.
    if me.p.y.abs() > HALF_WID - 3.0 {
        dirv.y -= me.p.y.signum() * 0.6;
        dirv = dirv.normalize_or(Vec2::new(dir, 0.0));
    }
    f.movement = dirv;
    let space = my_space > 4.0;
    f.sprint = Button { held: space || pressed_hard, pressed: false, released: false };
    // Flux sprint: Burst / Smog Step when a defender blocks the lane.
    if flux_ok && near_d < 3.5 && near_d > 1.0 && rng.chance(0.35 * diff) {
        f.flux = true;
        f.sprint = pressed();
        if sim.teams[team].kind == FluxKind::Smog {
            // Step past the defender, slightly to the side.
            let side = me.ai_dribble_side;
            let perp = Vec2::new(-dirv.y, dirv.x) * side;
            f.movement = (dirv + perp * 0.5).normalize_or(dirv);
        }
    }
    f
}
