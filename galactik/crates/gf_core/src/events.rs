use crate::flux::FluxAction;
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Restart {
    KickOff,
    GoalKick,
    Corner,
    ReEntry,
    FreeKick,
    Penalty,
}

/// Everything presentation needs to know, in the order it happened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimEvent {
    Whistle,
    KickOff { team: usize },
    Kick { player: usize, power: f32, kind: crate::input::KickKind, flux: bool },
    Bounce { pos: Vec2, speed: f32 },
    Possession { player: usize },
    Goal { team: usize, scorer: usize },
    OutOfPlay { restart: Restart, team: usize, at: Vec2 },
    Tackle { by: usize, on: usize, success: bool },
    Foul { by: usize, on: usize, at: Vec2 },
    Save { keeper: usize, caught: bool },
    FluxStart { player: usize, action: FluxAction },
    FluxFizzle { player: usize },
    Duel { attacker: usize, defender: usize, winner: usize },
    Teleport { player: usize, from: Vec2, to: Vec2 },
    Cloud { team: usize, at: Vec2, radius: f32 },
    TeamFlux { team: usize },
    Burnout { player: usize },
    Sick { player: usize },
    HalfTime,
    FullTime,
}
