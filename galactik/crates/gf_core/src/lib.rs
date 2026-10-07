//! Galactik Football simulation core. Engine-free and deterministic.

pub mod ai;
pub mod ball;
pub mod data;
pub mod events;
pub mod flux;
pub mod input;
pub mod pitch;
pub mod player;
pub mod rng;
pub mod sim;

pub use data::{builtin_teams, FluxKind, Pos, Stats, TeamDef};
pub use events::{Restart, SimEvent};
pub use flux::FluxAction;
pub use input::{Button, InputFrame, KickCmd, KickKind};
pub use player::{Action, PlayerState};
pub use sim::{MatchConfig, Phase, Sim, TeamState, DT};

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn scripted_input(tick: u64) -> InputFrame {
        let mut f = InputFrame::default();
        let a = (tick as f32) * 0.01;
        f.movement = Vec2::new(a.cos(), (a * 0.7).sin());
        f.sprint.held = tick % 200 < 100;
        f.sprint.pressed = tick % 200 == 0;
        f.flux = tick % 600 < 300;
        f.pass.held = tick % 90 < 20;
        f.pass.released = tick % 90 == 20;
        f.shoot.held = tick % 250 < 40;
        f.shoot.released = tick % 250 == 40;
        f.action_a.pressed = tick % 37 == 0;
        f
    }

    fn run(seed: u64, ticks: u64) -> (u64, Sim) {
        let teams = builtin_teams();
        let cfg = MatchConfig { seed, half_len_secs: 120.0, difficulty: 0.7, human: [true, false] };
        let mut sim = Sim::new(teams[0].clone(), teams[1].clone(), cfg);
        for t in 0..ticks {
            sim.step([Some(scripted_input(t)), None]);
        }
        (sim.state_hash(), sim)
    }

    #[test]
    fn teams_parse() {
        let teams = builtin_teams();
        assert_eq!(teams[0].starters().len(), 7);
        assert_eq!(teams[1].starters().len(), 7);
        assert_eq!(teams[0].players[0].name, "Ahito");
        assert_eq!(teams[1].flux, FluxKind::Smog);
    }

    #[test]
    fn deterministic() {
        let (a, _) = run(42, 60 * 60);
        let (b, _) = run(42, 60 * 60);
        assert_eq!(a, b);
        let (c, _) = run(43, 60 * 60);
        assert_ne!(a, c);
    }

    #[test]
    fn stays_sane_for_a_full_match() {
        let teams = builtin_teams();
        let cfg = MatchConfig { seed: 3, half_len_secs: 150.0, difficulty: 0.8, human: [false, false] };
        let mut sim = Sim::new(teams[0].clone(), teams[1].clone(), cfg);
        let mut kicks = 0;
        let mut flux = 0;
        while !matches!(sim.phase, Phase::FullTime) {
            sim.step([None, None]);
            for e in &sim.events {
                match e {
                    SimEvent::Kick { .. } => kicks += 1,
                    SimEvent::FluxStart { .. } => flux += 1,
                    _ => {}
                }
            }
            assert!(sim.ball.p.is_finite(), "ball position not finite");
            assert!(sim.ball.p.x.abs() < pitch::HALF_LEN + 3.0 && sim.ball.p.y.abs() < pitch::HALF_WID + 3.0, "ball escaped: {:?}", sim.ball.p);
            for p in &sim.players {
                assert!(p.p.is_finite());
            }
            if sim.tick > 60 * 60 * 20 {
                panic!("match never ended");
            }
        }
        assert_eq!(sim.half, 2);
        assert!(kicks > 50, "too few kicks: {kicks}");
        assert!(flux > 0, "no flux used");
    }
}
