//! Title screen and match setup.

use crate::setup::hex;
use crate::{AppState, MatchSetup};
use bevy::prelude::*;

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Title), spawn_title)
            .add_systems(Update, (title_input, update_title).run_if(in_state(AppState::Title)));
    }
}

#[derive(Component)]
struct SetupText;

fn spawn_title(mut commands: Commands) {
    let font = |size: f32| TextFont::from_font_size(size);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.01, 0.05, 0.85)),
            DespawnOnExit(AppState::Title),
        ))
        .with_children(|c| {
            c.spawn((Text::new("GALACTIK FOOTBALL"), font(64.0), TextColor(hex("#9ad7ff"))));
            c.spawn((Text::new("Snow Kids  vs  Shadows"), font(26.0), TextColor(Color::WHITE)));
            c.spawn((SetupText, Text::new(""), font(20.0), TextColor(hex("#ffe08a")), TextLayout::new_with_justify(Justify::Center)));
            c.spawn((
                Text::new(
                    "← / → or A / D : choose your team      T : two players      1 / 2 / 3 : 2, 3 or 5 minute halves      - / = : difficulty\n\nEnter or Start : kick off\n\n\
                     Hold Space (RT) with any action to use your Flux. Sprint = Breath Burst / Smog Step, shoot = Akillian Strike / Smog Shot,\n\
                     pass = Ice Lane / Veiled Pass, lob = High Breath / Phase Lob, tackle = Frost Lock / Smog Snatch. U = Super Jump / Eclipse Cloud.\n\
                     A full Flux pool unlocks the Team Flux (Y): Akillian Blizzard or Eclipse. Flux strains players; the Smog makes non-natives sick.",
                ),
                font(15.0),
                TextColor(Color::srgb(0.75, 0.8, 0.9)),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
}

fn title_input(keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut setup: ResMut<MatchSetup>, mut next: ResMut<NextState<AppState>>) {
    let mut left = keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA);
    let mut right = keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD);
    let mut start = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    for g in &pads {
        left |= g.just_pressed(GamepadButton::DPadLeft);
        right |= g.just_pressed(GamepadButton::DPadRight);
        start |= g.just_pressed(GamepadButton::Start) || g.just_pressed(GamepadButton::South);
        if g.just_pressed(GamepadButton::Select) {
            setup.two_players = !setup.two_players;
        }
    }
    if left || right {
        setup.human_team = 1 - setup.human_team;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        setup.two_players = !setup.two_players;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        setup.half_len_secs = 120.0;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        setup.half_len_secs = 180.0;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        setup.half_len_secs = 300.0;
    }
    if keys.just_pressed(KeyCode::Minus) {
        setup.difficulty = (setup.difficulty - 0.1).max(0.1);
    }
    if keys.just_pressed(KeyCode::Equal) {
        setup.difficulty = (setup.difficulty + 0.1).min(1.0);
    }
    if start {
        next.set(AppState::Match);
    }
}

fn update_title(setup: Res<MatchSetup>, mut q: Query<&mut Text, With<SetupText>>) {
    let Ok(mut t) = q.single_mut() else { return };
    let team = if setup.human_team == 0 { "Snow Kids (the Breath)" } else { "Shadows (the Smog)" };
    let players = if setup.two_players { "two players (P2: arrows + numpad, or second gamepad)" } else { "one player vs AI" };
    t.0 = format!(
        "You play: {}     {}     halves: {} min     AI difficulty: {:.0}%",
        team,
        players,
        (setup.half_len_secs / 60.0).round() as u32,
        setup.difficulty * 100.0
    );
}
