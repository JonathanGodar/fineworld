use bevy::{input::mouse::MouseMotion, prelude::*, render::render_graph::GraphInputNode};
use bevy_rapier3d::prelude::*;

use crate::{
    camera::{self, failed_camera::FailedCameraBundle},
    chunk::chunk_manager::ChunkManager,
    CollisionLayers, MainCamera,
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
            CollisionGroups::new(Group::GROUP_1, Group::empty()),
            // CollisionGroups::new(CollisionLayers::PLAYER.bits, CollisionLayers::all().bits),
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
        } else if keys.pressed(KeyCode::S) {
            controlled_velocity.z = 0.5;
        }

        if keys.pressed(KeyCode::Space) && kinematic_output.grounded {
            environment_velocity.y = 1.;
        }

        *controlled_velocity = ControlledVelocity(transform.rotation * controlled_velocity.0);
    }
}

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
    mut chunk_manager: ResMut<ChunkManager>,
) {
    let transform = q
        .get_single()
        .expect("Found no camera in block_break_system");

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    if let Some((entity, ray_intersect)) = rapier_ctx.cast_ray_and_get_normal(
        transform.translation(),
        transform.forward(),
        100.,
        true,
        QueryFilter::new().groups(InteractionGroups::default().with_filter(0b10.into())),
    ) {
        let point_inside_block = ray_intersect.point + ray_intersect.normal * -0.01;
        chunk_manager.break_block(point_inside_block.as_ivec3());

        // ray_intersect.point -

        // println!("{:?}, {:?}", entity, toi)
    }
}
