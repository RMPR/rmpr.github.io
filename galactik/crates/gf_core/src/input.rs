//! The only thing the simulation ever sees from the outside world.

use glam::Vec2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Button {
    pub held: bool,
    pub pressed: bool,
    pub released: bool,
}

impl Button {
    pub fn update(&mut self, down: bool) {
        self.pressed = down && !self.held;
        self.released = !down && self.held;
        self.held = down;
    }
}

/// One tick of intent for one controlled player. Humans fill the buttons;
/// the AI fills `kick` and `tackle` directly. Both go through the same code.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputFrame {
    /// Desired movement in pitch coordinates, length 0..=1.
    pub movement: Vec2,
    pub sprint: Button,
    /// Flux modifier held.
    pub flux: bool,
    /// Manual modifier held (raw aim for passes and shots).
    pub manual: bool,
    pub pass: Button,
    pub lob: Button,
    pub through: Button,
    pub shoot: Button,
    /// Switch player (attacking) / standing tackle (defending).
    pub action_a: Button,
    /// Slide tackle when defending.
    pub slide: Button,
    pub switch: Button,
    /// Smog only: deploy an Eclipse Cloud. Breath: Super Jump (aerial).
    pub special: Button,
    /// Team Flux (captain's ultimate), needs a full pool.
    pub team_flux: Button,
    /// Direct kick command, used by the AI and by dead-ball restarts.
    pub kick: Option<KickCmd>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KickKind {
    Pass,
    Through,
    Lob,
    Shot,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KickCmd {
    pub kind: KickKind,
    /// 0..=1
    pub power: f32,
    /// Aim direction in pitch coordinates (unit or zero).
    pub dir: Vec2,
    /// Explicit target teammate (AI), otherwise chosen from `dir`.
    pub target: Option<usize>,
    pub flux: bool,
    /// Raw aim instead of assisted targeting.
    pub manual: bool,
}
