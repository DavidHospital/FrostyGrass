use std::f32::consts::PI;

use bevy::{prelude::*, DefaultPlugins};
use bevy_xpbd_3d::prelude::PhysicsPlugins;
use terrain::TerrainPlugin;

mod render;
mod sampling;
mod terrain;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TerrainPlugin, PhysicsPlugins::default()))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, move_camera)
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10., 3., -10.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        ..default()
    });
}

fn move_camera(
    mut camera_q: Query<&mut Transform, With<Camera>>,
    keycodes: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut camera_transform = camera_q.single_mut();
    let mut velocity = Vec3::ZERO;
    let speed = time.delta_seconds() * 10.;
    let flat_forward = Vec3::Y.cross(camera_transform.forward().cross(Vec3::Y));
    if keycodes.pressed(KeyCode::W) {
        velocity = flat_forward * speed;
    }
    if keycodes.pressed(KeyCode::S) {
        velocity += -flat_forward * speed;
    }
    if keycodes.pressed(KeyCode::A) {
        velocity += camera_transform.left() * speed;
    }
    if keycodes.pressed(KeyCode::D) {
        velocity += camera_transform.right() * speed;
    }
    if keycodes.pressed(KeyCode::Space) {
        velocity += Vec3::Y * speed;
    }
    if keycodes.pressed(KeyCode::ShiftLeft) {
        velocity += -Vec3::Y * speed;
    }
    if velocity != Vec3::ZERO {
        camera_transform.translation += velocity;
    }

    let mut rotation = 0.;
    let rotate_speed = time.delta_seconds() * 6.;
    if keycodes.pressed(KeyCode::Q) {
        rotation += rotate_speed;
    }
    if keycodes.pressed(KeyCode::E) {
        rotation += -rotate_speed;
    }
    camera_transform.rotate_y(rotation);
}
