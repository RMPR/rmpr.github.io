//! Galactik Football: Bevy front end. The simulation lives in `gf_core`;
//! this crate turns inputs into `InputFrame`s, steps the sim at 60 Hz, and
//! renders whatever the sim says happened.

mod hud;
mod input;
mod setup;
mod sim_plugin;
mod title;
mod vfx;
mod view;

use bevy::prelude::*;
use bevy::window::WindowResolution;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Title,
    Match,
}

/// Ordering for the systems that build a match: the sim resource must exist
/// before anything spawns views of it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MatchSets {
    Init,
    Spawn,
}

/// Settings chosen on the title screen.
#[derive(Resource, Debug, Clone)]
pub struct MatchSetup {
    pub human_team: usize,
    pub two_players: bool,
    pub half_len_secs: f32,
    pub difficulty: f32,
    /// AI vs AI, for watching.
    pub spectate: bool,
    /// Skip the title screen (native `--match`).
    pub auto_match: bool,
}

impl Default for MatchSetup {
    fn default() -> Self {
        MatchSetup { human_team: 0, two_players: false, half_len_secs: 180.0, difficulty: 0.6, spectate: false, auto_match: false }
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    // Native dev flags: `--match` skips the title, `--ai` makes both teams AI.
    let mut setup = MatchSetup::default();
    #[cfg(not(target_arch = "wasm32"))]
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--match" => setup.auto_match = true,
            "--ai" => setup.spectate = true,
            "--shadows" => setup.human_team = 1,
            _ => {}
        }
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Galactik Football".into(),
                        resolution: WindowResolution::new(1280, 720),
                        canvas: Some("#gf-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.015, 0.015, 0.045)))
        .insert_resource(Time::<Fixed>::from_hz(gf_core::sim::DT.recip() as f64))
        .insert_resource(setup)
        .init_state::<AppState>()
        .configure_sets(OnEnter(AppState::Match), (MatchSets::Init, MatchSets::Spawn).chain())
        .add_plugins((
            setup::SetupPlugin,
            title::TitlePlugin,
            sim_plugin::SimPlugin,
            input::InputPlugin,
            view::ViewPlugin,
            hud::HudPlugin,
            vfx::VfxPlugin,
        ))
        .run();
}
