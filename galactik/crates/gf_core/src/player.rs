use crate::data::{Pos, Stats};
use crate::flux::FluxAction;
use crate::input::KickCmd;
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Idle,
    Kick { t: f32 },
    Tackle { t: f32, dir: Vec2, flux: bool, resolved: bool },
    Slide { t: f32, dir: Vec2, resolved: bool },
    Stunned { t: f32 },
    Frozen { t: f32 },
    /// Charging a Flux action; the stored kick fires when the wind-up ends.
    WindUp { t: f32, action: FluxAction, dir: Vec2, kick: Option<KickCmd> },
    Dive { t: f32 },
    Jump { t: f32 },
}

impl Action {
    pub fn locked(&self) -> bool {
        !matches!(self, Action::Idle | Action::Kick { .. })
    }
    pub fn can_act(&self) -> bool {
        matches!(self, Action::Idle)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActiveFlux {
    pub action: FluxAction,
    pub remaining: f32,
    pub started_at: f32,
}

impl Default for FluxAction {
    fn default() -> Self {
        FluxAction::Burst
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerState {
    pub id: usize,
    pub team: usize,
    pub def_index: usize,
    pub name: String,
    pub number: u8,
    pub pos: Pos,
    pub native: bool,
    pub stats: Stats,
    pub p: Vec2,
    pub v: Vec2,
    pub facing: Vec2,
    /// 0..=1
    pub stamina: f32,
    /// 0..=100
    pub strain: f32,
    pub burned_out: bool,
    pub sick: bool,
    pub sick_timer: f32,
    pub action: Action,
    pub flux: Option<ActiveFlux>,
    pub flux_cooldown: f32,
    /// Hold timers for kick power, per button.
    pub hold_pass: f32,
    pub hold_lob: f32,
    pub hold_through: f32,
    pub hold_shoot: f32,
    pub sprint_ball_timer: f32,
    pub press_timer: f32,
    /// For the AI: next sim time at which to re-decide.
    pub next_decision: f32,
    pub ai_dribble_side: f32,
}

impl PlayerState {
    pub fn is_gk(&self) -> bool {
        self.pos == Pos::GK
    }

    pub fn fatigued(&self) -> bool {
        self.strain >= crate::flux::STRAIN_FATIGUE
    }

    pub fn flux_active(&self, a: FluxAction) -> bool {
        matches!(self.flux, Some(f) if f.action == a)
    }

    pub fn can_use_flux(&self) -> bool {
        !self.burned_out && self.flux.is_none() && self.flux_cooldown <= 0.0
    }
}
