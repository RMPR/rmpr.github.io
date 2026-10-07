//! Camera, lights and shared assets that live for the whole app.

use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::AmbientLight;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_camera, build_palette));
    }
}

#[derive(Component)]
pub struct MainCamera;

/// Shared meshes and materials.
#[derive(Resource)]
pub struct Palette {
    pub capsule: Handle<Mesh>,
    pub head: Handle<Mesh>,
    pub nose: Handle<Mesh>,
    pub ring: Handle<Mesh>,
    pub ball: Handle<Mesh>,
    pub aura: Handle<Mesh>,
    pub particle: Handle<Mesh>,
    pub cloud: Handle<Mesh>,
    pub line_x: Handle<Mesh>,
    pub ball_mat: Handle<StandardMaterial>,
    pub ball_breath_mat: Handle<StandardMaterial>,
    pub ball_smog_mat: Handle<StandardMaterial>,
    pub ring_mat: Handle<StandardMaterial>,
    pub ring_p2_mat: Handle<StandardMaterial>,
    pub aura_breath: Handle<StandardMaterial>,
    pub aura_smog: Handle<StandardMaterial>,
    pub cloud_mat: Handle<StandardMaterial>,
    pub frost_mat: Handle<StandardMaterial>,
    pub smoke_mat: Handle<StandardMaterial>,
    pub spark_mat: Handle<StandardMaterial>,
    pub nose_mat: Handle<StandardMaterial>,
    pub line_mat: Handle<StandardMaterial>,
    pub pitch_mat: Handle<StandardMaterial>,
    pub post_mat: Handle<StandardMaterial>,
    pub net_mat: Handle<StandardMaterial>,
    pub wall_mat: Handle<StandardMaterial>,
    pub stand_mat: Handle<StandardMaterial>,
}

pub fn hex(s: &str) -> Color {
    Srgba::hex(s).map(Color::from).unwrap_or(Color::WHITE)
}

/// A readable colour for a team in UI text: the kit's primary unless it is
/// too dark, then the secondary, then the accent.
pub fn team_ui_color(kit: &gf_core::data::Kit) -> Color {
    for c in [&kit.primary, &kit.secondary, &kit.accent] {
        let col = hex(c);
        let lin: LinearRgba = col.into();
        if lin.luminance() > 0.08 {
            return col;
        }
    }
    Color::WHITE
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Hdr,
        Bloom { intensity: 0.22, ..Bloom::NATURAL },
        Tonemapping::TonyMcMapface,
        Projection::from(PerspectiveProjection { fov: 0.62, ..default() }),
        Transform::from_xyz(0.0, 26.0, 34.0).looking_at(Vec3::new(0.0, 0.0, 2.0), Vec3::Y),
        AmbientLight { color: Color::srgb(0.6, 0.7, 1.0), brightness: 400.0, affects_lightmapped_meshes: true },
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.05, 0.5, 0.0)),
    ));
}

fn build_palette(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let emissive = |mats: &mut Assets<StandardMaterial>, c: Color, strength: f32, alpha: Option<f32>| {
        let lin: LinearRgba = c.into();
        mats.add(StandardMaterial {
            base_color: if let Some(a) = alpha { c.with_alpha(a) } else { c },
            emissive: LinearRgba::rgb(lin.red * strength, lin.green * strength, lin.blue * strength),
            unlit: alpha.is_some(),
            alpha_mode: if alpha.is_some() { AlphaMode::Blend } else { AlphaMode::Opaque },
            perceptual_roughness: 0.9,
            ..default()
        })
    };
    let plain = |mats: &mut Assets<StandardMaterial>, c: Color| {
        mats.add(StandardMaterial { base_color: c, perceptual_roughness: 0.95, metallic: 0.0, ..default() })
    };
    let pal = Palette {
        capsule: meshes.add(Capsule3d::new(0.4, 1.0)),
        head: meshes.add(Sphere::new(0.3)),
        nose: meshes.add(Cuboid::new(0.18, 0.18, 0.5)),
        ring: meshes.add(Torus::new(0.55, 0.75)),
        ball: meshes.add(Sphere::new(0.24)),
        aura: meshes.add(Sphere::new(1.0)),
        particle: meshes.add(Sphere::new(0.12)),
        cloud: meshes.add(Sphere::new(1.0)),
        line_x: meshes.add(Cuboid::new(1.0, 0.03, 0.14)),
        ball_mat: emissive(&mut mats, Color::srgb(1.0, 1.0, 1.0), 0.6, None),
        ball_breath_mat: emissive(&mut mats, Color::srgb(0.5, 0.85, 1.0), 6.0, None),
        ball_smog_mat: emissive(&mut mats, Color::srgb(0.55, 0.2, 0.9), 5.0, None),
        ring_mat: emissive(&mut mats, Color::srgb(1.0, 0.9, 0.2), 3.0, None),
        ring_p2_mat: emissive(&mut mats, Color::srgb(1.0, 0.35, 0.2), 3.0, None),
        aura_breath: emissive(&mut mats, Color::srgb(0.45, 0.8, 1.0), 2.5, Some(0.22)),
        aura_smog: emissive(&mut mats, Color::srgb(0.35, 0.05, 0.5), 1.5, Some(0.35)),
        cloud_mat: emissive(&mut mats, Color::srgb(0.05, 0.02, 0.08), 0.1, Some(0.5)),
        frost_mat: emissive(&mut mats, Color::srgb(0.7, 0.92, 1.0), 5.0, None),
        smoke_mat: emissive(&mut mats, Color::srgb(0.12, 0.03, 0.18), 0.6, None),
        spark_mat: emissive(&mut mats, Color::srgb(1.0, 0.85, 0.4), 7.0, None),
        nose_mat: plain(&mut mats, Color::srgb(0.1, 0.1, 0.1)),
        line_mat: emissive(&mut mats, Color::srgb(0.55, 0.9, 1.0), 4.0, None),
        pitch_mat: plain(&mut mats, Color::srgb(0.07, 0.10, 0.22)),
        post_mat: emissive(&mut mats, Color::srgb(0.9, 0.95, 1.0), 3.0, None),
        net_mat: emissive(&mut mats, Color::srgb(0.5, 0.8, 1.0), 0.8, Some(0.12)),
        wall_mat: emissive(&mut mats, Color::srgb(0.3, 0.7, 1.0), 1.2, Some(0.05)),
        stand_mat: plain(&mut mats, Color::srgb(0.05, 0.06, 0.1)),
    };
    commands.insert_resource(pal);
}
