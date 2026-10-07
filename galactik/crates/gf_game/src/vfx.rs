//! Cheap event-driven effects: emissive particles for Flux and kicks, and
//! stadium-wide colour for Team Flux.

use crate::setup::{MainCamera, Palette};
use crate::sim_plugin::{GameEvent, MatchRes};
use crate::view::to_world;
use crate::AppState;
use bevy::light::AmbientLight;
use bevy::prelude::*;
use gf_core::{FluxAction, FluxKind, SimEvent};

pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fx>()
            .add_systems(Update, (spawn_from_events, trails, tick_particles, stadium_mood).run_if(in_state(AppState::Match)));
    }
}

#[derive(Resource, Default)]
struct Fx {
    seed: u32,
}

impl Fx {
    fn rand(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }
    fn sym(&mut self) -> f32 {
        self.rand() * 2.0 - 1.0
    }
}

#[derive(Component)]
struct Particle {
    vel: Vec3,
    life: f32,
    max: f32,
    gravity: f32,
}

fn burst(commands: &mut Commands, fx: &mut Fx, pal: &Palette, mat: Handle<StandardMaterial>, at: Vec3, count: usize, speed: f32, life: f32, gravity: f32) {
    for _ in 0..count {
        let dir = Vec3::new(fx.sym(), fx.rand() * 0.8 + 0.2, fx.sym()).normalize_or_zero();
        let s = speed * (0.5 + fx.rand());
        commands.spawn((
            Particle { vel: dir * s, life, max: life, gravity },
            Mesh3d(pal.particle.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(at).with_scale(Vec3::splat(0.6 + fx.rand() * 0.8)),
            DespawnOnExit(AppState::Match),
        ));
    }
}

fn spawn_from_events(mut commands: Commands, mut events: MessageReader<GameEvent>, m: Res<MatchRes>, pal: Res<Palette>, mut fx: ResMut<Fx>) {
    let sim = &m.sim;
    for GameEvent(e) in events.read() {
        match *e {
            SimEvent::FluxStart { player, action } => {
                let p = &sim.players[player];
                let at = to_world(p.p, 1.0);
                let (mat, n, speed) = match action.kind() {
                    FluxKind::Breath => (pal.frost_mat.clone(), 26, 5.0),
                    FluxKind::Smog => (pal.smoke_mat.clone(), 30, 3.5),
                };
                burst(&mut commands, &mut fx, &pal, mat, at, n, speed, 0.7, if action.kind() == FluxKind::Breath { 6.0 } else { -1.5 });
            }
            SimEvent::Teleport { player, from, to } => {
                let kind = sim.teams[sim.players[player].team].kind;
                let mat = if kind == FluxKind::Smog { pal.smoke_mat.clone() } else { pal.frost_mat.clone() };
                burst(&mut commands, &mut fx, &pal, mat.clone(), to_world(from, 1.0), 24, 3.0, 0.8, -1.0);
                burst(&mut commands, &mut fx, &pal, mat, to_world(to, 1.0), 24, 3.0, 0.8, -1.0);
            }
            SimEvent::Kick { player, power, flux, .. } => {
                let p = &sim.players[player];
                let at = to_world(p.p + p.facing * 0.5, 0.2);
                if flux {
                    let kind = sim.teams[p.team].kind;
                    let mat = if kind == FluxKind::Smog { pal.smoke_mat.clone() } else { pal.frost_mat.clone() };
                    burst(&mut commands, &mut fx, &pal, mat, at, 30, 7.0, 0.8, 2.0);
                } else if power > 0.6 {
                    burst(&mut commands, &mut fx, &pal, pal.spark_mat.clone(), at, 6, 3.0, 0.35, 8.0);
                }
            }
            SimEvent::Goal { team, .. } => {
                let dir = sim.attack_dir(team);
                let at = to_world(Vec2::new(gf_core::pitch::HALF_LEN * dir, 0.0), 1.5);
                burst(&mut commands, &mut fx, &pal, pal.spark_mat.clone(), at, 120, 10.0, 1.8, 6.0);
            }
            SimEvent::Duel { attacker, defender, .. } => {
                for id in [attacker, defender] {
                    let p = &sim.players[id];
                    burst(&mut commands, &mut fx, &pal, pal.spark_mat.clone(), to_world(p.p, 1.2), 20, 5.0, 0.9, 0.0);
                }
            }
            SimEvent::Save { keeper, .. } => {
                let p = &sim.players[keeper];
                burst(&mut commands, &mut fx, &pal, pal.spark_mat.clone(), to_world(p.p, 1.0), 10, 4.0, 0.5, 6.0);
            }
            SimEvent::Tackle { by, success: true, .. } => {
                let p = &sim.players[by];
                burst(&mut commands, &mut fx, &pal, pal.spark_mat.clone(), to_world(p.p, 0.3), 6, 2.5, 0.4, 6.0);
            }
            SimEvent::TeamFlux { team } => {
                let mat = match sim.teams[team].kind {
                    FluxKind::Breath => pal.frost_mat.clone(),
                    FluxKind::Smog => pal.smoke_mat.clone(),
                };
                for &id in &sim.teams[team].players {
                    let p = &sim.players[id];
                    burst(&mut commands, &mut fx, &pal, mat.clone(), to_world(p.p, 1.0), 20, 6.0, 1.2, 1.0);
                }
            }
            _ => {}
        }
    }
}

/// Continuous trails: Breath Burst frost wake, ice-lane ball, smog clouds.
fn trails(mut commands: Commands, m: Res<MatchRes>, pal: Res<Palette>, mut fx: ResMut<Fx>, time: Res<Time>) {
    let sim = &m.sim;
    let frame_ok = (time.elapsed_secs() * 60.0) as u32 % 2 == 0;
    if !frame_ok {
        return;
    }
    for p in &sim.players {
        if p.flux_active(FluxAction::Burst) {
            burst(&mut commands, &mut fx, &pal, pal.frost_mat.clone(), to_world(p.p - p.facing * 0.4, 0.4), 2, 1.0, 0.5, 0.0);
        }
        if p.flux_active(FluxAction::SmogSnatch) || p.flux_active(FluxAction::FrostLock) {
            let kind = sim.teams[p.team].kind;
            let mat = if kind == FluxKind::Smog { pal.smoke_mat.clone() } else { pal.frost_mat.clone() };
            burst(&mut commands, &mut fx, &pal, mat, to_world(p.p, 0.6), 2, 1.5, 0.4, -0.5);
        }
    }
    let b = &sim.ball;
    if b.flags.ice_lane || b.flags.flux_shot || b.flags.high_breath {
        let kind = b.last_team.map(|t| sim.teams[t].kind).unwrap_or(FluxKind::Breath);
        let mat = if kind == FluxKind::Smog { pal.smoke_mat.clone() } else { pal.frost_mat.clone() };
        burst(&mut commands, &mut fx, &pal, mat, Vec3::new(b.p.x, b.p.z + 0.1, b.p.y), 2, 0.6, 0.45, 0.0);
    }
    for c in &sim.clouds {
        burst(&mut commands, &mut fx, &pal, pal.smoke_mat.clone(), to_world(c.p + Vec2::new(fx.sym(), fx.sym()) * c.r * 0.8, 0.3), 2, 1.0, 1.0, -1.2);
    }
}

fn tick_particles(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Particle, &mut Transform)>) {
    let dt = time.delta_secs();
    for (e, mut p, mut tf) in &mut q {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        p.vel.y -= p.gravity * dt;
        let v = p.vel;
        tf.translation += v * dt;
        if tf.translation.y < 0.05 {
            tf.translation.y = 0.05;
            p.vel.y = p.vel.y.abs() * 0.3;
        }
        let k = p.life / p.max;
        tf.scale = Vec3::splat(k.max(0.05)) * 1.0;
    }
}

fn stadium_mood(m: Res<MatchRes>, mut clear: ResMut<ClearColor>, mut cam: Query<&mut AmbientLight, With<MainCamera>>) {
    let sim = &m.sim;
    let now = sim.time;
    let blizzard = sim.teams.iter().any(|t| t.team_flux_active(now) && t.kind == FluxKind::Breath);
    let eclipse = sim.teams.iter().any(|t| t.team_flux_active(now) && t.kind == FluxKind::Smog);
    let (bg, amb, bright) = if eclipse {
        (Color::srgb(0.01, 0.0, 0.02), Color::srgb(0.35, 0.1, 0.5), 120.0)
    } else if blizzard {
        (Color::srgb(0.05, 0.08, 0.16), Color::srgb(0.7, 0.85, 1.0), 900.0)
    } else {
        (Color::srgb(0.015, 0.015, 0.045), Color::srgb(0.6, 0.7, 1.0), 400.0)
    };
    clear.0 = bg;
    if let Ok(mut a) = cam.single_mut() {
        a.color = amb;
        a.brightness = bright;
    }
}
