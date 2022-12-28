use bevy::{input::mouse::MouseMotion, prelude::*};

#[derive(Component)]
pub struct FailedCamera;

#[derive(Bundle)]
pub struct FailedCameraBundle {
    pub failed_camera: FailedCamera,
    pub camera_bundle: Camera3dBundle,
}

#[derive(Default)]
pub struct FailedCameraPlugin {}

impl Plugin for FailedCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(camera_movement_system);
    }
}

pub fn camera_pitch_system(
    mut q: Query<&mut Transform, With<FailedCamera>>,
    mut mouse_evr: EventReader<MouseMotion>,
) {
    let mut transform = q.get_single_mut().unwrap();

    let mouse_motion = mouse_evr.iter().fold(Vec2::ZERO, |acc, ev| acc + ev.delta);
    let sensitivity = 0.0009;

    transform.rotate_local_x(-mouse_motion.y * sensitivity);
}

pub fn camera_movement_system(
    mut q: Query<&mut Transform, With<FailedCamera>>,
    keys: Res<Input<KeyCode>>,
    mut mouse_evr: EventReader<MouseMotion>,
) {
    let mut transform = q.get_single_mut().unwrap();

    let mut any_movement_requested = false;
    let mut wanted_move = Vec3::ZERO;

    // Forward is -z
    if keys.pressed(KeyCode::W) {
        wanted_move += transform.forward();
        any_movement_requested = true;
    } else if keys.pressed(KeyCode::S) {
        wanted_move += transform.back();
        any_movement_requested = true;
    }

    // Do nothing if A and D are pressed together
    if keys.pressed(KeyCode::A) && keys.pressed(KeyCode::D) {
    } else if keys.pressed(KeyCode::D) {
        wanted_move += transform.right();
        any_movement_requested = true;
    } else if keys.pressed(KeyCode::A) {
        wanted_move += transform.left();
        any_movement_requested = true;
    };

    // Do nothing if Space and Shift are pressed together
    if keys.pressed(KeyCode::Space) && keys.pressed(KeyCode::LShift) {
    } else if keys.pressed(KeyCode::LShift) {
        wanted_move += transform.down();
        any_movement_requested = true;
    } else if keys.pressed(KeyCode::Space) {
        wanted_move += transform.up();
        any_movement_requested = true;
    };

    let mouse_motion = mouse_evr.iter().fold(Vec2::ZERO, |acc, ev| acc + ev.delta);
    let sensitivity = 0.0009;

    transform.rotate_local_y(-mouse_motion.x * sensitivity);
    transform.rotate_local_x(-mouse_motion.y * sensitivity);

    if !any_movement_requested {
        return;
    }

    wanted_move = wanted_move.normalize();
    transform.translation += wanted_move * 0.2;
}
