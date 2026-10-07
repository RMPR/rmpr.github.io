//! In-match HUD: score, clock, Flux charges, strain, banners.

use crate::setup::{hex, team_ui_color};
use crate::sim_plugin::{GameEvent, MatchRes, Paused};
use crate::AppState;
use bevy::prelude::*;
use gf_core::flux::CHARGE;
use gf_core::{Phase, Restart, SimEvent};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Banner>()
            .add_systems(OnEnter(AppState::Match), spawn_hud.in_set(crate::MatchSets::Spawn))
            .add_systems(Update, (banner_from_events, update_hud, back_to_title).run_if(in_state(AppState::Match)));
    }
}

#[derive(Resource, Default)]
pub struct Banner {
    pub text: String,
    pub sub: String,
    pub until: f32,
    pub color: Color,
}

impl Banner {
    fn show(&mut self, now: f32, secs: f32, text: impl Into<String>, sub: impl Into<String>, color: Color) {
        self.text = text.into();
        self.sub = sub.into();
        self.until = now + secs;
        self.color = color;
    }
}

#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct ClockText;
#[derive(Component)]
struct BannerText;
#[derive(Component)]
struct BannerSub;
#[derive(Component)]
struct Charge {
    team: usize,
    index: u32,
}
#[derive(Component)]
struct StrainBar;
#[derive(Component)]
struct PlayerName;
#[derive(Component)]
struct PoolLabel {
    team: usize,
}

fn spawn_hud(mut commands: Commands, m: Res<MatchRes>) {
    let scoped = DespawnOnExit(AppState::Match);
    let font = |size: f32| TextFont::from_font_size(size);
    let t0 = &m.sim.teams[0].def;
    let t1 = &m.sim.teams[1].def;

    // Top bar.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.0),
                ..default()
            },
            scoped.clone(),
        ))
        .with_children(|c| {
            c.spawn((
                Node { padding: UiRect::axes(Val::Px(18.0), Val::Px(4.0)), ..default() },
                BackgroundColor(Color::srgba(0.0, 0.02, 0.1, 0.75)),
            ))
            .with_children(|c| {
                c.spawn((ScoreText, Text::new(format!("{} 0 - 0 {}", t0.short_name, t1.short_name)), font(30.0), TextColor(Color::WHITE)));
            });
            c.spawn((ClockText, Text::new("1st  00:00"), font(18.0), TextColor(hex("#9ad7ff"))));
        });

    // Flux charges: left for team 0, right for team 1.
    for team in 0..2 {
        let def = &m.sim.teams[team].def;
        let mut node = Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        };
        if team == 0 {
            node.left = Val::Px(16.0);
        } else {
            node.right = Val::Px(16.0);
            node.align_items = AlignItems::FlexEnd;
        }
        commands.spawn((node, scoped.clone())).with_children(|c| {
            c.spawn((Text::new(def.name.clone()), font(18.0), TextColor(team_ui_color(&def.kit))));
            c.spawn((PoolLabel { team }, Text::new("Flux"), font(13.0), TextColor(Color::srgb(0.7, 0.8, 0.9))));
            c.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), ..default() }).with_children(|c| {
                for i in 0..3 {
                    c.spawn((
                        Charge { team, index: i },
                        Node { width: Val::Px(34.0), height: Val::Px(10.0), ..default() },
                        BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
                    ));
                }
            });
        });
    }

    // Controlled player name + strain, bottom left.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            scoped.clone(),
        ))
        .with_children(|c| {
            c.spawn((PlayerName, Text::new(""), font(18.0), TextColor(Color::WHITE)));
            c.spawn((Node { width: Val::Px(160.0), height: Val::Px(8.0), ..default() }, BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.6)))).with_children(|c| {
                c.spawn((StrainBar, Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(hex("#ffb347"))));
            });
            c.spawn((Text::new("strain"), font(12.0), TextColor(Color::srgb(0.6, 0.6, 0.7))));
        });

    // Centre banner.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(30.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                ..default()
            },
            scoped.clone(),
        ))
        .with_children(|c| {
            c.spawn((BannerText, Text::new(""), font(56.0), TextColor(Color::WHITE), TextLayout::new(Justify::Center, LineBreak::WordBoundary)));
            c.spawn((BannerSub, Text::new(""), font(22.0), TextColor(Color::srgb(0.85, 0.9, 1.0)), TextLayout::new(Justify::Center, LineBreak::WordBoundary)));
        });

    // Controls hint, bottom right.
    commands.spawn((
        Node { position_type: PositionType::Absolute, bottom: Val::Px(16.0), right: Val::Px(16.0), ..default() },
        Text::new("WASD move  J pass/tackle  K shoot/slide  L lob  I through  Shift sprint  Space+action = FLUX  U special  Y team flux  Q switch  Esc pause"),
        font(12.0),
        TextColor(Color::srgba(0.7, 0.75, 0.85, 0.8)),
        scoped,
    ));
}

fn banner_from_events(mut events: MessageReader<GameEvent>, m: Res<MatchRes>, time: Res<Time>, mut banner: ResMut<Banner>) {
    let now = time.elapsed_secs();
    let sim = &m.sim;
    for GameEvent(e) in events.read() {
        match *e {
            SimEvent::Goal { team, scorer } => {
                let t = &sim.teams[team].def;
                banner.show(now, 3.0, "GOAL!", format!("{} - {}", sim.players[scorer].name, t.name), team_ui_color(&t.kit));
            }
            SimEvent::Foul { by, on, .. } => {
                banner.show(now, 1.6, "FOUL", format!("{} on {}", sim.players[by].name, sim.players[on].name), Color::srgb(1.0, 0.75, 0.3));
            }
            SimEvent::Duel { attacker, defender, winner } => {
                let loser = if winner == attacker { defender } else { attacker };
                banner.show(now, 1.8, "FLUX DUEL", format!("{} beats {}", sim.players[winner].name, sim.players[loser].name), Color::srgb(1.0, 0.95, 0.5));
            }
            SimEvent::TeamFlux { team } => {
                banner.show(now, 2.5, sim.team_flux_name(team).to_uppercase(), sim.teams[team].def.name.clone(), team_ui_color(&sim.teams[team].def.kit));
            }
            SimEvent::FluxStart { player, action } => {
                if banner.until < now + 0.2 {
                    let t = &sim.teams[sim.players[player].team].def;
                    banner.show(now, 0.9, "", format!("{}: {}", sim.players[player].name, action.name()), team_ui_color(&t.kit));
                }
            }
            SimEvent::Burnout { player } => banner.show(now, 1.5, "", format!("{} is burned out", sim.players[player].name), Color::srgb(1.0, 0.5, 0.4)),
            SimEvent::Sick { player } => banner.show(now, 1.5, "", format!("{} suffers from the Smog", sim.players[player].name), Color::srgb(0.8, 0.5, 1.0)),
            SimEvent::OutOfPlay { restart, team, .. } => {
                let label = match restart {
                    Restart::Corner => "Corner",
                    Restart::GoalKick => "Goal kick",
                    Restart::ReEntry => "Re-entry",
                    Restart::FreeKick => "Free kick",
                    Restart::Penalty => "PENALTY",
                    Restart::KickOff => "Kick-off",
                };
                banner.show(now, 1.2, "", format!("{} - {}", label, sim.teams[team].def.name), Color::srgb(0.8, 0.9, 1.0));
            }
            SimEvent::HalfTime => banner.show(now, 3.0, "HALF TIME", "", Color::WHITE),
            SimEvent::FullTime => banner.show(now, 999.0, "FULL TIME", "Enter / Start to return to the title", Color::WHITE),
            SimEvent::Save { keeper, caught } => {
                if banner.until < now {
                    banner.show(now, 0.8, "", format!("{}: {}", sim.players[keeper].name, if caught { "caught it" } else { "parried" }), Color::srgb(0.8, 0.9, 1.0));
                }
            }
            _ => {}
        }
    }
}

fn update_hud(
    m: Res<MatchRes>,
    paused: Res<Paused>,
    banner: Res<Banner>,
    time: Res<Time>,
    mut score: Query<&mut Text, (With<ScoreText>, Without<ClockText>, Without<BannerText>, Without<BannerSub>, Without<PlayerName>, Without<PoolLabel>)>,
    mut clock: Query<&mut Text, (With<ClockText>, Without<ScoreText>, Without<BannerText>, Without<BannerSub>, Without<PlayerName>, Without<PoolLabel>)>,
    mut btext: Query<(&mut Text, &mut TextColor), (With<BannerText>, Without<ScoreText>, Without<ClockText>, Without<BannerSub>, Without<PlayerName>, Without<PoolLabel>)>,
    mut bsub: Query<&mut Text, (With<BannerSub>, Without<ScoreText>, Without<ClockText>, Without<BannerText>, Without<PlayerName>, Without<PoolLabel>)>,
    mut name: Query<&mut Text, (With<PlayerName>, Without<ScoreText>, Without<ClockText>, Without<BannerText>, Without<BannerSub>, Without<PoolLabel>)>,
    mut pools: Query<(&PoolLabel, &mut Text), (Without<ScoreText>, Without<ClockText>, Without<BannerText>, Without<BannerSub>, Without<PlayerName>)>,
    mut charges: Query<(&Charge, &mut BackgroundColor)>,
    mut strain: Query<&mut Node, With<StrainBar>>,
) {
    let sim = &m.sim;
    let now = time.elapsed_secs();
    if let Ok(mut t) = score.single_mut() {
        t.0 = format!("{} {} - {} {}", sim.teams[0].def.short_name, sim.teams[0].score, sim.teams[1].score, sim.teams[1].def.short_name);
    }
    if let Ok(mut t) = clock.single_mut() {
        let secs = sim.clock.min(sim.cfg.half_len_secs) as u32;
        let half = if sim.half == 1 { "1st" } else { "2nd" };
        let extra = match sim.phase {
            Phase::FullTime => "  FULL TIME",
            Phase::HalfTime { .. } => "  HALF TIME",
            _ if paused.0 => "  PAUSED",
            _ => "",
        };
        t.0 = format!("{}  {:02}:{:02}{}", half, secs / 60, secs % 60, extra);
    }
    let show = banner.until > now || paused.0;
    if let Ok((mut t, mut c)) = btext.single_mut() {
        t.0 = if paused.0 { "PAUSED".into() } else if show { banner.text.clone() } else { String::new() };
        c.0 = banner.color;
    }
    if let Ok(mut t) = bsub.single_mut() {
        t.0 = if show && !paused.0 { banner.sub.clone() } else { String::new() };
    }
    for (pl, mut t) in &mut pools {
        let ts = &sim.teams[pl.team];
        let tf = if ts.team_flux_active(sim.time) { format!("  {}!", sim.team_flux_name(pl.team)) } else { String::new() };
        t.0 = format!("Flux {:>3}{}", ts.pool as u32, tf);
    }
    for (ch, mut bg) in &mut charges {
        let ts = &sim.teams[ch.team];
        let fill = ((ts.pool - ch.index as f32 * CHARGE) / CHARGE).clamp(0.0, 1.0);
        let col = team_ui_color(&ts.def.kit);
        bg.0 = if fill >= 1.0 {
            col
        } else if fill > 0.0 {
            col.with_alpha(0.25 + 0.5 * fill)
        } else {
            Color::srgba(0.3, 0.3, 0.4, 0.5)
        };
    }
    // Controlled player of the first human team.
    let human = (0..2).find(|&t| sim.teams[t].human);
    if let Some(t) = human {
        if let Some(id) = sim.teams[t].controlled {
            let p = &sim.players[id];
            if let Ok(mut n) = name.single_mut() {
                let status = if p.burned_out { " (burned out)" } else if p.sick { " (sick)" } else if p.fatigued() { " (fatigued)" } else { "" };
                n.0 = format!("{} {} #{}{}", p.pos.label(), p.name, p.number, status);
            }
            if let Ok(mut node) = strain.single_mut() {
                node.width = Val::Percent(p.strain);
            }
        }
    }
}

fn back_to_title(m: Res<MatchRes>, keys: Res<ButtonInput<KeyCode>>, pads: Query<&Gamepad>, mut next: ResMut<NextState<AppState>>) {
    if !matches!(m.sim.phase, Phase::FullTime) {
        return;
    }
    let pad = pads.iter().any(|g| g.just_pressed(GamepadButton::Start) || g.just_pressed(GamepadButton::South));
    if keys.just_pressed(KeyCode::Enter) || pad {
        next.set(AppState::Title);
    }
}
