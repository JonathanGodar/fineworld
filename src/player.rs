use bevy::{input::mouse::MouseMotion, prelude::*, render::render_graph::GraphInputNode};
use bevy_rapier3d::prelude::*;

use crate::{
    camera::{self, failed_camera::FailedCameraBundle},
    MainCamera, chunk::{self, LoadedChunks, Chunk, LoadedChunk},
};

#[derive(Component, Default)]
pub struct Player {}

#[derive(Component, Default, Deref, DerefMut)]
pub struct CharacterVelocity(Vec3);

// pub fn create_player(
//     mut material: ResMut<Assets<StandardMaterial>>,
// ) {
// }

pub fn add_gravity(
    mut commands: Commands,
    player: Query<Entity, (With<Player>, Without<Gravity>)>,
    chunks: Query<&Chunk, Added<LoadedChunk>>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut material: ResMut<Assets<StandardMaterial>>,
){
    if let Ok(player) = player.get_single(){
        if chunks.iter().find(|chnk| chnk.chunk_coords == IVec3::new(0, 1, 0)).is_none() {
            return;
        }
    
        commands.entity(player).insert(Gravity::default());
    }
}

pub fn setup_player_system(
    mut commands: Commands,
    player: Query<&Player>,
    // chunks: Query<&Chunk, Added<LoadedChunk>>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut material: ResMut<Assets<StandardMaterial>>,

) {
    // if !player.is_empty() { return; }
    // if chunks.iter().find(|chnk| chnk.chunk_coords == IVec3::new(0, 1, 0)).is_none() {
    //     return;
    // }

    // // chunk
    // chunk_evr.iter().find_map(|chnk| chnk)
    // // chunk_evr.iter().find(|chnk| chunks[chunk_evr]){

    // }
    
    // for evt in chunk_loaded

    let child = commands
        .spawn((
            FailedCameraBundle {
                camera_bundle: Camera3dBundle {
                    transform: Transform::from_xyz(0., 5., 10.).looking_at(Vec3::ZERO, Vec3::Y),
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
            // RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.5, 0.5),
            KinematicCharacterController::default(),
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
            CharacterVelocity::default(),
        ))
        .add_child(child);
}

const MOVEMENT_SPEED: f32 = 0.005;
const MAX_MOVEMENT_SPEED: Vec3 = Vec3::new(0.02, f32::MAX, 0.02);
pub fn player_movement_system(
    q: Query<&Transform, With<Player>>
){
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
pub struct Gravity{
    acceleration: Vec3,
    prev_vel: Vec3,
}

const GRAVITY_CONSTANT: Vec3 = Vec3::new(0., -0.00982, 0.);

impl Default for Gravity {
    fn default() -> Self {
        Self {
            acceleration: GRAVITY_CONSTANT,
            prev_vel: Vec3::ZERO,
        }
    }
}

pub fn gravity_system(
    mut q: Query<(
        // &mut CharacterVelocity,
        &mut KinematicCharacterController,
        &mut Gravity,
        &KinematicCharacterControllerOutput,
    )>,
) {
    for (mut controller, mut gravity, output) in q.iter_mut() {
        let acceleration = gravity.acceleration;
        if output.grounded {
            gravity.prev_vel = Vec3::ZERO;
        } else {
            gravity.prev_vel += acceleration;
            controller.translation = Some(gravity.prev_vel);
            println!("hello! {:?}", controller.translation);
            
            // transform.translation += gravity.prev_vel;
        }
    }
    // for mut entity in q.
    // for mut entity in q.iter_mut() {
    //     if !entity.2.grounded && entity.1 .0.y < 0. {
    //         entity.0 .0 += entity.1 .0;
    //     }
    // }
}

pub fn character_velocity_system(
    mut q: Query<(&mut KinematicCharacterController, &CharacterVelocity)>,
) {
    // for entity in q.iter_mut() {
    //     let (mut controller, velocity) = entity;
    //     controller.translation = Some(**velocity);
    // }
}