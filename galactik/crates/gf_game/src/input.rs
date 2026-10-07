//! Keyboard and gamepad mapping to `InputFrame`s. Edges are computed per
//! fixed tick so the simulation sees consistent press/release events.

use crate::sim_plugin::Paused;
use crate::{AppState, MatchSetup};
use bevy::prelude::*;
use gf_core::InputFrame;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HumanInputs>()
            .add_systems(FixedUpdate, sample_inputs.run_if(in_state(AppState::Match)).before(crate::sim_plugin::SimStep))
            .add_systems(Update, pause_toggle.run_if(in_state(AppState::Match)));
    }
}

/// Raw booleans for one controller, before edge detection.
#[derive(Clone, Copy, Default, Debug)]
pub struct Raw {
    pub movement: Vec2,
    pub sprint: bool,
    pub flux: bool,
    pub manual: bool,
    pub pass: bool,
    pub lob: bool,
    pub through: bool,
    pub shoot: bool,
    pub switch: bool,
    pub special: bool,
    pub team_flux: bool,
}

#[derive(Resource, Default)]
pub struct HumanInputs {
    pub frames: [InputFrame; 2],
    /// Which human slot (0 = player one) controls each team.
    pub slot_for_team: [Option<usize>; 2],
}

impl HumanInputs {
    pub fn frame_for_team(&self, team: usize) -> InputFrame {
        match self.slot_for_team[team] {
            Some(s) => self.frames[s],
            None => InputFrame::default(),
        }
    }
}

fn apply(frame: &mut InputFrame, raw: Raw) {
    frame.movement = raw.movement.clamp_length_max(1.0);
    frame.flux = raw.flux;
    frame.manual = raw.manual;
    frame.sprint.update(raw.sprint);
    frame.pass.update(raw.pass);
    frame.lob.update(raw.lob);
    frame.through.update(raw.through);
    frame.shoot.update(raw.shoot);
    // Pass doubles as tackle and shoot doubles as slide, like PES.
    frame.action_a.update(raw.pass);
    frame.slide.update(raw.shoot);
    frame.switch.update(raw.switch);
    frame.special.update(raw.special);
    frame.team_flux.update(raw.team_flux);
    frame.kick = None;
}

fn down(keys: &ButtonInput<KeyCode>, k: KeyCode) -> bool {
    keys.pressed(k) || keys.just_pressed(k)
}

fn keyboard_p1(keys: &ButtonInput<KeyCode>) -> Raw {
    let mut m = Vec2::ZERO;
    if down(keys, KeyCode::KeyW) { m.y -= 1.0; }
    if down(keys, KeyCode::KeyS) { m.y += 1.0; }
    if down(keys, KeyCode::KeyA) { m.x -= 1.0; }
    if down(keys, KeyCode::KeyD) { m.x += 1.0; }
    Raw {
        movement: m,
        sprint: down(keys, KeyCode::ShiftLeft) || down(keys, KeyCode::ShiftRight),
        flux: down(keys, KeyCode::Space),
        manual: down(keys, KeyCode::ControlLeft),
        pass: down(keys, KeyCode::KeyJ),
        shoot: down(keys, KeyCode::KeyK),
        lob: down(keys, KeyCode::KeyL),
        through: down(keys, KeyCode::KeyI),
        switch: down(keys, KeyCode::KeyQ),
        special: down(keys, KeyCode::KeyU),
        team_flux: down(keys, KeyCode::KeyY),
    }
}

fn keyboard_p2(keys: &ButtonInput<KeyCode>) -> Raw {
    let mut m = Vec2::ZERO;
    if down(keys, KeyCode::ArrowUp) { m.y -= 1.0; }
    if down(keys, KeyCode::ArrowDown) { m.y += 1.0; }
    if down(keys, KeyCode::ArrowLeft) { m.x -= 1.0; }
    if down(keys, KeyCode::ArrowRight) { m.x += 1.0; }
    Raw {
        movement: m,
        sprint: down(keys, KeyCode::Numpad0),
        flux: down(keys, KeyCode::NumpadEnter),
        manual: down(keys, KeyCode::NumpadDecimal),
        pass: down(keys, KeyCode::Numpad1),
        shoot: down(keys, KeyCode::Numpad2),
        lob: down(keys, KeyCode::Numpad3),
        through: down(keys, KeyCode::Numpad5),
        switch: down(keys, KeyCode::Numpad4),
        special: down(keys, KeyCode::Numpad6),
        team_flux: down(keys, KeyCode::Numpad7),
    }
}

fn gdown(g: &Gamepad, b: GamepadButton) -> bool {
    g.pressed(b) || g.just_pressed(b)
}

fn gamepad_raw(g: &Gamepad) -> Raw {
    let stick = g.left_stick();
    let dead = if stick.length() < 0.18 { Vec2::ZERO } else { stick };
    let dpad = g.dpad();
    let mut m = Vec2::new(dead.x, -dead.y);
    if m.length() < 0.1 && (dpad.x.abs() > 0.5 || dpad.y.abs() > 0.5) {
        m = Vec2::new(dpad.x, -dpad.y);
    }
    Raw {
        movement: m,
        sprint: gdown(g, GamepadButton::RightTrigger),
        flux: gdown(g, GamepadButton::RightTrigger2),
        manual: gdown(g, GamepadButton::LeftTrigger2),
        pass: gdown(g, GamepadButton::South),
        shoot: gdown(g, GamepadButton::West),
        lob: gdown(g, GamepadButton::East),
        through: gdown(g, GamepadButton::North),
        switch: gdown(g, GamepadButton::LeftTrigger),
        special: gdown(g, GamepadButton::DPadUp) || gdown(g, GamepadButton::RightThumb),
        team_flux: gdown(g, GamepadButton::DPadDown) || gdown(g, GamepadButton::LeftThumb),
    }
}

fn merge(a: Raw, b: Raw) -> Raw {
    Raw {
        movement: if a.movement.length() > 0.1 { a.movement } else { b.movement },
        sprint: a.sprint || b.sprint,
        flux: a.flux || b.flux,
        manual: a.manual || b.manual,
        pass: a.pass || b.pass,
        lob: a.lob || b.lob,
        through: a.through || b.through,
        shoot: a.shoot || b.shoot,
        switch: a.switch || b.switch,
        special: a.special || b.special,
        team_flux: a.team_flux || b.team_flux,
    }
}

fn sample_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<(Entity, &Gamepad)>,
    setup: Res<MatchSetup>,
    mut inputs: ResMut<HumanInputs>,
) {
    let mut pads: Vec<(Entity, &Gamepad)> = pads.iter().collect();
    pads.sort_by_key(|(e, _)| e.index());
    let p1 = pads.first().map(|(_, g)| gamepad_raw(g)).unwrap_or_default();
    let p2 = pads.get(1).map(|(_, g)| gamepad_raw(g)).unwrap_or_default();
    let raw1 = merge(keyboard_p1(&keys), p1);
    let raw2 = merge(keyboard_p2(&keys), p2);
    let mut frames = inputs.frames;
    apply(&mut frames[0], raw1);
    apply(&mut frames[1], raw2);
    inputs.frames = frames;
    inputs.slot_for_team = [None, None];
    inputs.slot_for_team[setup.human_team] = Some(0);
    if setup.two_players {
        inputs.slot_for_team[1 - setup.human_team] = Some(1);
    }
}

fn pause_toggle(keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut paused: ResMut<Paused>) {
    let pad_start = pads.iter().any(|g| g.just_pressed(GamepadButton::Start));
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyP) || pad_start {
        paused.0 = !paused.0;
    }
}
