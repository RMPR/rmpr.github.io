//! Spawns the pitch, the players and the ball, and keeps them in sync with
//! the simulation with interpolation between fixed ticks.

use crate::setup::{hex, MainCamera, Palette};
use crate::sim_plugin::{MatchRes, Paused};
use crate::AppState;
use bevy::prelude::*;
use gf_core::pitch::*;
use gf_core::{Action, FluxKind, Phase};

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Match), (spawn_pitch, spawn_actors).chain().in_set(crate::MatchSets::Spawn))
            .add_systems(Update, (sync_actors, sync_clouds, follow_camera).chain().run_if(in_state(AppState::Match)));
    }
}

/// Sim (x, y, z-up) to Bevy (x, y-up, z).
pub fn to_world(p: Vec2, h: f32) -> Vec3 {
    Vec3::new(p.x, h, p.y)
}

#[derive(Component)]
pub struct PlayerView {
    pub id: usize,
}

#[derive(Component)]
pub struct Aura;

#[derive(Component)]
pub struct ControlRing {
    pub team: usize,
}

#[derive(Component)]
pub struct BallView;

#[derive(Component)]
pub struct CloudView {
    pub index: usize,
}

#[derive(Component, Clone)]
pub struct MatchEntity;

fn spawn_pitch(mut commands: Commands, pal: Res<Palette>, mut meshes: ResMut<Assets<Mesh>>) {
    let scoped = (MatchEntity, DespawnOnExit(AppState::Match));
    // Ground.
    let ground = meshes.add(Plane3d::default().mesh().size(HALF_LEN * 2.0 + 16.0, HALF_WID * 2.0 + 16.0));
    commands.spawn((Mesh3d(ground), MeshMaterial3d(pal.pitch_mat.clone()), Transform::from_xyz(0.0, -0.02, 0.0), scoped.clone()));

    let line = |commands: &mut Commands, a: Vec2, b: Vec2| {
        let d = b - a;
        let len = d.length();
        let mid = (a + b) * 0.5;
        let angle = -d.y.atan2(d.x);
        commands.spawn((
            Mesh3d(pal.line_x.clone()),
            MeshMaterial3d(pal.line_mat.clone()),
            Transform { translation: to_world(mid, 0.01), rotation: Quat::from_rotation_y(angle), scale: Vec3::new(len, 1.0, 1.0) },
            scoped.clone(),
        ));
    };
    let l = HALF_LEN;
    let w = HALF_WID;
    // Boundary, halfway line, boxes.
    line(&mut commands, Vec2::new(-l, -w), Vec2::new(l, -w));
    line(&mut commands, Vec2::new(-l, w), Vec2::new(l, w));
    line(&mut commands, Vec2::new(-l, -w), Vec2::new(-l, w));
    line(&mut commands, Vec2::new(l, -w), Vec2::new(l, w));
    line(&mut commands, Vec2::new(0.0, -w), Vec2::new(0.0, w));
    for s in [-1.0f32, 1.0] {
        let bx = s * (l - BOX_DEPTH);
        line(&mut commands, Vec2::new(s * l, -BOX_HALF_WIDTH), Vec2::new(bx, -BOX_HALF_WIDTH));
        line(&mut commands, Vec2::new(s * l, BOX_HALF_WIDTH), Vec2::new(bx, BOX_HALF_WIDTH));
        line(&mut commands, Vec2::new(bx, -BOX_HALF_WIDTH), Vec2::new(bx, BOX_HALF_WIDTH));
        // Penalty spot.
        line(&mut commands, Vec2::new(s * (l - PENALTY_SPOT) - 0.15, 0.0), Vec2::new(s * (l - PENALTY_SPOT) + 0.15, 0.0));
    }
    // Centre circle.
    let segs = 48;
    for i in 0..segs {
        let a0 = i as f32 / segs as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / segs as f32 * std::f32::consts::TAU;
        line(&mut commands, Vec2::new(a0.cos(), a0.sin()) * CENTRE_CIRCLE_R, Vec2::new(a1.cos(), a1.sin()) * CENTRE_CIRCLE_R);
    }

    // Goals.
    let post = meshes.add(Cylinder::new(0.08, GOAL_HEIGHT));
    let bar = meshes.add(Cylinder::new(0.08, GOAL_HALF_WIDTH * 2.0 + 0.16));
    let net = meshes.add(Cuboid::new(GOAL_DEPTH, GOAL_HEIGHT, GOAL_HALF_WIDTH * 2.0));
    for s in [-1.0f32, 1.0] {
        let gx = s * l;
        for y in [-GOAL_HALF_WIDTH, GOAL_HALF_WIDTH] {
            commands.spawn((Mesh3d(post.clone()), MeshMaterial3d(pal.post_mat.clone()), Transform::from_translation(to_world(Vec2::new(gx, y), GOAL_HEIGHT / 2.0)), scoped.clone()));
        }
        commands.spawn((
            Mesh3d(bar.clone()),
            MeshMaterial3d(pal.post_mat.clone()),
            Transform { translation: to_world(Vec2::new(gx, 0.0), GOAL_HEIGHT), rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), ..default() },
            scoped.clone(),
        ));
        commands.spawn((
            Mesh3d(net.clone()),
            MeshMaterial3d(pal.net_mat.clone()),
            Transform::from_translation(to_world(Vec2::new(gx + s * GOAL_DEPTH / 2.0, 0.0), GOAL_HEIGHT / 2.0)),
            scoped.clone(),
        ));
    }
    // Holographic side walls and stands.
    let wall_long = meshes.add(Cuboid::new(HALF_LEN * 2.0 + 2.0, 3.0, 0.05));
    let wall_short = meshes.add(Cuboid::new(0.05, 3.0, HALF_WID * 2.0 + 2.0));
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(wall_long.clone()), MeshMaterial3d(pal.wall_mat.clone()), Transform::from_translation(to_world(Vec2::new(0.0, s * (w + 1.0)), 1.5)), scoped.clone()));
        commands.spawn((Mesh3d(wall_short.clone()), MeshMaterial3d(pal.wall_mat.clone()), Transform::from_translation(to_world(Vec2::new(s * (l + 1.0), 0.0), 1.5)), scoped.clone()));
    }
    let stand = meshes.add(Cuboid::new(HALF_LEN * 2.0 + 20.0, 6.0, 6.0));
    let stand_short = meshes.add(Cuboid::new(6.0, 6.0, HALF_WID * 2.0 + 8.0));
    for s in [-1.0f32, 1.0] {
        commands.spawn((Mesh3d(stand.clone()), MeshMaterial3d(pal.stand_mat.clone()), Transform::from_translation(to_world(Vec2::new(0.0, s * (w + 7.0)), 3.0)), scoped.clone()));
        commands.spawn((Mesh3d(stand_short.clone()), MeshMaterial3d(pal.stand_mat.clone()), Transform::from_translation(to_world(Vec2::new(s * (l + 7.0), 0.0), 3.0)), scoped.clone()));
    }
}

fn spawn_actors(mut commands: Commands, pal: Res<Palette>, m: Res<MatchRes>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let scoped = (MatchEntity, DespawnOnExit(AppState::Match));
    for t in 0..2 {
        let def = &m.sim.teams[t].def;
        let kit = mats.add(StandardMaterial { base_color: hex(&def.kit.primary), perceptual_roughness: 0.8, ..default() });
        let gk_kit = mats.add(StandardMaterial { base_color: hex(&def.kit.accent), perceptual_roughness: 0.8, ..default() });
        for &id in &m.sim.teams[t].players {
            let p = &m.sim.players[id];
            let pd = &def.players[p.def_index];
            let head = mats.add(StandardMaterial { base_color: hex(pd.accent.as_deref().unwrap_or("#333333")), perceptual_roughness: 0.7, ..default() });
            let body = if p.is_gk() { gk_kit.clone() } else { kit.clone() };
            let aura_mat = match def.flux {
                FluxKind::Breath => pal.aura_breath.clone(),
                FluxKind::Smog => pal.aura_smog.clone(),
            };
            commands
                .spawn((PlayerView { id }, Transform::from_translation(to_world(p.p, 0.0)), Visibility::default(), scoped.clone()))
                .with_children(|c| {
                    c.spawn((Mesh3d(pal.capsule.clone()), MeshMaterial3d(body), Transform::from_xyz(0.0, 0.9, 0.0)));
                    c.spawn((Mesh3d(pal.head.clone()), MeshMaterial3d(head), Transform::from_xyz(0.0, 1.55, 0.0)));
                    c.spawn((Mesh3d(pal.nose.clone()), MeshMaterial3d(pal.nose_mat.clone()), Transform::from_xyz(0.0, 1.0, 0.5)));
                    c.spawn((Aura, Mesh3d(pal.aura.clone()), MeshMaterial3d(aura_mat), Transform::from_xyz(0.0, 0.9, 0.0).with_scale(Vec3::splat(1.3)), Visibility::Hidden));
                });
        }
        if m.sim.teams[t].human {
            let mat = if t == 0 { pal.ring_mat.clone() } else { pal.ring_p2_mat.clone() };
            commands.spawn((
                ControlRing { team: t },
                Mesh3d(pal.ring.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(0.0, 0.05, 0.0),
                scoped.clone(),
            ));
        }
    }
    commands.spawn((BallView, Mesh3d(pal.ball.clone()), MeshMaterial3d(pal.ball_mat.clone()), Transform::from_xyz(0.0, 0.24, 0.0), scoped.clone()));
}

fn sync_actors(
    m: Res<MatchRes>,
    fixed: Res<Time<Fixed>>,
    pal: Res<Palette>,
    mut players: Query<(&PlayerView, &mut Transform, &Children), (Without<BallView>, Without<ControlRing>)>,
    mut auras: Query<(&mut Visibility, &mut Transform), (With<Aura>, Without<PlayerView>, Without<BallView>, Without<ControlRing>)>,
    mut ball: Query<(&mut Transform, &mut MeshMaterial3d<StandardMaterial>), (With<BallView>, Without<PlayerView>, Without<ControlRing>)>,
    mut rings: Query<(&ControlRing, &mut Transform, &mut Visibility), (Without<PlayerView>, Without<BallView>, Without<Aura>)>,
    time: Res<Time>,
) {
    let alpha = fixed.overstep_fraction();
    let sim = &m.sim;
    let now = time.elapsed_secs();
    for (pv, mut tf, children) in &mut players {
        let p = &sim.players[pv.id];
        let (pp, pf) = m.prev_players[pv.id];
        // Teleports should not interpolate.
        let pos = if pp.distance(p.p) > 3.0 { p.p } else { pp.lerp(p.p, alpha) };
        let facing = if pf.dot(p.facing) < 0.0 { p.facing } else { pf.lerp(p.facing, alpha).normalize_or(p.facing) };
        tf.translation = to_world(pos, 0.0);
        let yaw = Quat::from_rotation_y((-facing.y).atan2(facing.x) - std::f32::consts::FRAC_PI_2);
        let tilt = match p.action {
            Action::Slide { .. } => Quat::from_rotation_x(1.2),
            Action::Dive { .. } => Quat::from_rotation_x(1.1),
            Action::Stunned { .. } => Quat::from_rotation_x(0.9),
            Action::Frozen { .. } => Quat::from_rotation_z(0.15),
            Action::Jump { t } => Quat::IDENTITY * Quat::from_rotation_x(0.0 * t),
            Action::Tackle { .. } => Quat::from_rotation_x(0.45),
            Action::Kick { t } => Quat::from_rotation_x(0.25 * (1.0 - t / 0.3).max(0.0)),
            Action::WindUp { t, .. } => Quat::from_rotation_x(-0.2 - 0.1 * (t * 30.0).sin()),
            _ => Quat::IDENTITY,
        };
        tf.rotation = yaw * tilt;
        let jump = match p.action {
            Action::Jump { t } => (t / 0.6 * std::f32::consts::PI).sin() * 1.5,
            _ => 0.0,
        };
        tf.translation.y = jump;
        tf.scale = if p.burned_out { Vec3::new(1.0, 0.9, 1.0) } else { Vec3::ONE };

        for child in children.iter() {
            if let Ok((mut vis, mut atf)) = auras.get_mut(child) {
                let active = p.flux.is_some() || matches!(p.action, Action::WindUp { .. });
                *vis = if active { Visibility::Inherited } else { Visibility::Hidden };
                let pulse = 1.25 + 0.15 * (now * 14.0).sin();
                atf.scale = Vec3::splat(pulse);
            }
        }
    }
    for (ring, mut tf, mut vis) in &mut rings {
        match sim.teams[ring.team].controlled {
            Some(id) => {
                let p = &sim.players[id];
                let (pp, _) = m.prev_players[id];
                let pos = if pp.distance(p.p) > 3.0 { p.p } else { pp.lerp(p.p, alpha) };
                tf.translation = to_world(pos, 0.06);
                tf.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                let strain = p.strain / 100.0;
                tf.scale = Vec3::splat(1.0 + 0.1 * (now * 6.0).sin()) * (1.0 - 0.25 * strain);
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    if let Ok((mut tf, mut mat)) = ball.single_mut() {
        let b = &sim.ball;
        let pos = if m.prev_ball.distance(b.p) > 3.0 { b.p } else { m.prev_ball.lerp(b.p, alpha) };
        tf.translation = Vec3::new(pos.x, pos.z + 0.13, pos.y);
        let spin = Quat::from_rotation_z(-now * 6.0 * b.v.length().min(10.0) * 0.1) * Quat::from_rotation_x(now * 5.0);
        tf.rotation = spin;
        let want = if b.flags.flux_shot || b.flags.ice_lane || b.flags.high_breath {
            match b.last_team.map(|t| sim.teams[t].kind) {
                Some(FluxKind::Smog) => pal.ball_smog_mat.clone(),
                _ => pal.ball_breath_mat.clone(),
            }
        } else if b.flags.phase || sim.ball_hidden() {
            pal.ball_smog_mat.clone()
        } else {
            pal.ball_mat.clone()
        };
        if mat.0 != want {
            mat.0 = want;
        }
        let hidden = sim.ball_hidden();
        tf.scale = if hidden { Vec3::splat(0.35) } else { Vec3::ONE };
    }
}

fn sync_clouds(mut commands: Commands, m: Res<MatchRes>, pal: Res<Palette>, mut clouds: Query<(Entity, &CloudView, &mut Transform)>) {
    let sim = &m.sim;
    let mut seen = vec![false; sim.clouds.len()];
    for (e, cv, mut tf) in &mut clouds {
        if let Some(c) = sim.clouds.get(cv.index) {
            seen[cv.index] = true;
            let r = c.r * (c.t / 3.0).clamp(0.3, 1.0).sqrt();
            tf.translation = to_world(c.p, 1.0);
            tf.scale = Vec3::new(r, r * 0.6, r);
        } else {
            commands.entity(e).despawn();
        }
    }
    for (i, c) in sim.clouds.iter().enumerate() {
        if !seen[i] {
            commands.spawn((
                CloudView { index: i },
                Mesh3d(pal.cloud.clone()),
                MeshMaterial3d(pal.cloud_mat.clone()),
                Transform::from_translation(to_world(c.p, 1.0)).with_scale(Vec3::splat(c.r)),
                MatchEntity,
                DespawnOnExit(AppState::Match),
            ));
        }
    }
}

fn follow_camera(m: Res<MatchRes>, paused: Res<Paused>, time: Res<Time>, fixed: Res<Time<Fixed>>, mut cam: Query<&mut Transform, With<MainCamera>>) {
    let Ok(mut tf) = cam.single_mut() else { return };
    let sim = &m.sim;
    let alpha = fixed.overstep_fraction();
    let b = m.prev_ball.lerp(sim.ball.p, alpha);
    let mut focus = Vec3::new(b.x, 0.0, b.y);
    // Lead the ball in its direction of travel a little.
    focus.x += sim.ball.v.x.clamp(-8.0, 8.0) * 0.35;
    focus.x = focus.x.clamp(-HALF_LEN + 6.0, HALF_LEN - 6.0);
    focus.z = (focus.z * 0.5).clamp(-8.0, 8.0);
    let (dist, height) = match sim.phase {
        Phase::Goal { .. } => (24.0, 14.0),
        _ => (34.0, 24.0),
    };
    let desired = focus + Vec3::new(0.0, height, dist);
    let dt = time.delta_secs();
    let k = if paused.0 { 0.0 } else if sim.slowmo > 0 { 1.0 - (-dt * 1.5).exp() } else { 1.0 - (-dt * 4.0).exp() };
    tf.translation = tf.translation.lerp(desired, k);
    let look = tf.translation + (focus + Vec3::new(0.0, 1.0, 0.0) - tf.translation);
    tf.look_at(look, Vec3::Y);
}
