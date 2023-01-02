use bevy::{input::mouse::MouseMotion, prelude::*, render::render_graph::GraphInputNode};
use bevy_rapier3d::prelude::*;

use crate::{
    camera::{self, failed_camera::FailedCameraBundle},
    chunk::{self, Chunk, LoadedChunk, LoadedChunks},
    MainCamera,
};

#[derive(Component, Default)]
pub struct Player {}

#[derive(Component, Default, Deref, DerefMut)]
// Controlled by the environment, eg. gravity
pub struct EnvironmentVelocity(Vec3);

#[derive(Component, Default, Deref, DerefMut)]
// Controlled by the controller of the entity
pub struct ControlledVelocity(Vec3);

pub fn setup_player_system(
    mut commands: Commands,
    player: Query<&Player>,
    // chunks: Query<&Chunk, Added<LoadedChunk>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut material: ResMut<Assets<StandardMaterial>>,
) {
    let child = commands
        .spawn((
            FailedCameraBundle {
                camera_bundle: Camera3dBundle {
                    transform: Transform::from_xyz(0., 1., 0.1).looking_at(Vec3::ZERO, Vec3::Y),
                    // transform: Transform::from_xyz(0., 5., 10.).looking_at(Vec3::ZERO, Vec3::Y),
                    ..default()
                },
                failed_camera: camera::failed_camera::FailedCamera,
            },
            MainCamera,
        ))
        .id();

    commands
        .spawn((
            Player::default(),
            Collider::capsule_y(0.5, 0.5),
            KinematicCharacterController::default(),
            Gravity::default(),
            PbrBundle {
                mesh: meshes.add(
                    shape::Capsule {
                        radius: 0.5,
                        depth: 1.0,
                        ..default()
                    }
                    .into(),
                ),
                material: material.add(StandardMaterial::from(Color::BLUE)),
                transform: Transform::from_xyz(7.5, 36., 7.5),
                ..Default::default()
            },
            EnvironmentVelocity::default(),
            ControlledVelocity::default(),
        ))
        .add_child(child);
}

const MOVEMENT_SPEED: f32 = 0.005;
const MAX_MOVEMENT_SPEED: Vec3 = Vec3::new(0.02, f32::MAX, 0.02);
pub fn player_movement_system(
    mut q: Query<
        (
            &mut ControlledVelocity,
            &mut EnvironmentVelocity,
            &mut Transform,
            &KinematicCharacterController,
            &KinematicCharacterControllerOutput,
        ),
        With<Player>,
    >,
    keys: Res<Input<KeyCode>>,
    mut mouse_evr: EventReader<MouseMotion>,
) {
    for (
        mut controlled_velocity,
        mut environment_velocity,
        mut transform,
        controller,
        kinematic_output,
    ) in q.iter_mut()
    {
        let mouse_motion = mouse_evr.iter().fold(Vec2::ZERO, |acc, ev| acc + ev.delta);
        let sensitivity = 0.0045;

        transform.rotate_local_y(-mouse_motion.x * sensitivity);

        controlled_velocity.x = 0.;
        controlled_velocity.z = 0.;

        let desired_vel = Vec3::ZERO;

        if keys.pressed(KeyCode::W) {
            controlled_velocity.z -= 0.5;
            // if velocity.z > -0.5 {
            //     velocity.z = -0.5;
            // }
            // velocity.z += -0
            // velocity.z = velocity.z.min(-0.5);
        }

        if keys.pressed(KeyCode::Space) && kinematic_output.grounded {
            environment_velocity.y = 1.;
        }

        *controlled_velocity = ControlledVelocity(transform.rotation * controlled_velocity.0);
    }
    // println!("{:?}", q.get_single().map(|t| t.translation));
}
// pub fn player_movement_system(
//     mut query: Query<
//         (
//             &mut Transform,
//             Option<&KinematicCharacterControllerOutput>,
//             // &mut CharacterVelocity,
//         ),
//         With<Player>,
//     >,
//     keys: Res<Input<KeyCode>>,
//     mut mouse_evr: EventReader<MouseMotion>,
// ) {
//     let (mut transform, kinematic_output, mut character_velocity) = query.single_mut();

//     let mut any_movement_requested = false;
//     let mut wanted_move = Vec3::ZERO;

//     // Forward is -z
//     if keys.pressed(KeyCode::W) {
//         wanted_move += transform.forward();
//         any_movement_requested = true;
//     } else if keys.pressed(KeyCode::S) {
//         wanted_move += transform.back();
//         any_movement_requested = true;
//     }

//     // Do nothing if A and D are pressed together
//     if keys.pressed(KeyCode::A) && keys.pressed(KeyCode::D) {
//     } else if keys.pressed(KeyCode::D) {
//         wanted_move += transform.right();
//         any_movement_requested = true;
//     } else if keys.pressed(KeyCode::A) {
//         wanted_move += transform.left();
//         any_movement_requested = true;
//     };

//     if keys.pressed(KeyCode::Space) && kinematic_output.map_or(false, |outp| outp.grounded) {
//         wanted_move += transform.up() * 10.;
//         any_movement_requested = true;
//     };

//     let mouse_motion = mouse_evr.iter().fold(Vec2::ZERO, |acc, ev| acc + ev.delta);
//     let sensitivity = 0.0045;

//     transform.rotate_local_y(-mouse_motion.x * sensitivity);

//     if !any_movement_requested {
//         return;
//     }

//     character_velocity.0 += (wanted_move * MOVEMENT_SPEED)
//         .min(MAX_MOVEMENT_SPEED)
//         .max(-MAX_MOVEMENT_SPEED);
// }

#[derive(Component)]
pub struct Gravity {
    acceleration: f32,
    prev_vel: f32,
    max_vel: f32,
}

const GRAVITY_CONSTANT: f32 = -0.00982;

impl Default for Gravity {
    fn default() -> Self {
        Self {
            acceleration: GRAVITY_CONSTANT,
            prev_vel: 0.,
            max_vel: -2.,
        }
    }
}

pub fn gravity_system(
    mut q: Query<(
        &mut EnvironmentVelocity,
        // &mut KinematicCharacterController,
        &mut Gravity,
        &KinematicCharacterControllerOutput,
    )>,
) {
    for (mut velocity, mut gravity, output) in q.iter_mut() {
        let acceleration = gravity.acceleration;
        if output.grounded {
            gravity.prev_vel = 0.;
        } else {
            gravity.prev_vel += acceleration;
            velocity.y = (gravity.prev_vel + velocity.y).max(gravity.max_vel);
        }
    }
}

pub fn character_velocity_system(
    mut q: Query<(
        &mut KinematicCharacterController,
        Option<&mut EnvironmentVelocity>,
        Option<&mut ControlledVelocity>,
    )>,
) {
    for (mut controller, mut environment_vel_opt, mut controlled_vel_opt) in q.iter_mut() {
        let mut vel_result = Vec3::ZERO;
        if let Some(controlled_vel) = controlled_vel_opt {
            vel_result += controlled_vel.0;
        }
        if let Some(environment_vel) = environment_vel_opt {
            vel_result += environment_vel.0;
        }

        controller.translation = Some(vel_result);
    }
}

pub fn block_break_system(
    mut q: Query<&GlobalTransform, With<MainCamera>>,
    rapier_ctx: Res<RapierContext>,
    mouse: Res<Input<MouseButton>>,
) {
    let transform = q
        .get_single()
        .expect("Found no camera in block_break_system");

    // QueryFilterFlags::

    if !mouse.pressed(MouseButton::Left) {
        // println!("Mouse not pressed");
        return;
    }

    if let Some((entity, toi)) = rapier_ctx.cast_ray_and_get_normal(
        transform.translation(),
        transform.forward(),
        100.,
        true,
        QueryFilter::new(),
    ) {
        println!("{:?}, {:?}", entity, toi)
    } else {
        println!("Cast ray, found nothing");
    }
}

pub fn print_player_pos(mut q: Query<&Transform, With<Player>>, keys: Res<Input<KeyCode>>) {
    if !keys.just_pressed(KeyCode::P) {
        return;
    }

    let transform = q.get_single();

    println!("PlayerPos: {:?}")
}
