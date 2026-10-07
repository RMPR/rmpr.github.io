//! Owns the simulation, steps it in `FixedUpdate`, and republishes its
//! events as Bevy messages.

use crate::input::HumanInputs;
use crate::{AppState, MatchSetup};
use bevy::prelude::*;
use gf_core::{builtin_teams, InputFrame, MatchConfig, Phase, Sim, SimEvent};

pub struct SimPlugin;

/// System set that steps the simulation; input sampling runs before it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimStep;

#[derive(Message)]
pub struct GameEvent(pub SimEvent);

#[derive(Resource, Default)]
pub struct Paused(pub bool);

/// The live match plus the previous tick for interpolation.
#[derive(Resource)]
pub struct MatchRes {
    pub sim: Sim,
    pub prev_players: Vec<(Vec2, Vec2)>,
    pub prev_ball: Vec3,
    pub steps_this_frame: u32,
}

impl MatchRes {
    pub fn new(sim: Sim) -> Self {
        let prev_players = sim.players.iter().map(|p| (p.p, p.facing)).collect();
        MatchRes { prev_ball: sim.ball.p, prev_players, sim, steps_this_frame: 0 }
    }
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameEvent>()
            .init_resource::<Paused>()
            .add_systems(OnEnter(AppState::Match), start_match.in_set(crate::MatchSets::Init))
            .add_systems(OnExit(AppState::Match), end_match)
            .add_systems(FixedUpdate, step_sim.in_set(SimStep).run_if(in_state(AppState::Match)));
    }
}

fn start_match(mut commands: Commands, setup: Res<MatchSetup>, mut paused: ResMut<Paused>) {
    let teams = builtin_teams();
    let mut human = [false, false];
    human[setup.human_team] = true;
    if setup.two_players {
        human = [true, true];
    }
    if setup.spectate {
        human = [false, false];
    }
    let seed = 1 + (setup.half_len_secs as u64) * 7 + setup.human_team as u64;
    let cfg = MatchConfig { half_len_secs: setup.half_len_secs, seed, difficulty: setup.difficulty, human };
    let sim = Sim::new(teams[0].clone(), teams[1].clone(), cfg);
    commands.insert_resource(MatchRes::new(sim));
    paused.0 = false;
}

fn end_match(mut commands: Commands) {
    commands.remove_resource::<MatchRes>();
}

fn step_sim(mut m: ResMut<MatchRes>, inputs: Res<HumanInputs>, paused: Res<Paused>, mut events: MessageWriter<GameEvent>) {
    if paused.0 || matches!(m.sim.phase, Phase::FullTime) {
        return;
    }
    let m = &mut *m;
    for (i, p) in m.sim.players.iter().enumerate() {
        m.prev_players[i] = (p.p, p.facing);
    }
    m.prev_ball = m.sim.ball.p;
    let mut frames: [Option<InputFrame>; 2] = [None, None];
    for t in 0..2 {
        if m.sim.teams[t].human {
            frames[t] = Some(inputs.frame_for_team(t));
        }
    }
    m.sim.step(frames);
    m.steps_this_frame += 1;
    for e in &m.sim.events {
        events.write(GameEvent(*e));
    }
}
