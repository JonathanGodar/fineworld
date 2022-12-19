use std::time::Duration;

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin, ScheduleRunnerSettings},
    prelude::*,
    render::camera::RenderTarget,
    sprite::collide_aabb::{collide, Collision},
    time::FixedTimestep,
    window::close_on_esc,
};

const TIME_STEP: f32 = 0.016;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        window: WindowDescriptor {
            monitor: MonitorSelection::Index(1),
            ..default()
        },
        ..default()
    }))
    .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.8)))
    .add_startup_system(setup_camera)
    .add_startup_system(setup_world)
    .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
    .add_system(close_on_esc)
    .run();
}

#[derive(Component)]
struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0., 5., -10.).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(MainCamera);
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Cube::new(5.).into()),
        material: materials.add(StandardMaterial::from(Color::GREEN)),
        ..Default::default()
    });
}

fn camera_movement_system(mut q: Query<&mut Transform, With<MainCamera>>) {}
