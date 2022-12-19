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
    .add_startup_system(setup)
    .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
    .add_system(close_on_esc)
    .run();
}

#[derive(Component)]
struct MainCamera;

fn setup(mut commands: Commands) {
    setup_camera(commands);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle::default()).insert(MainCamera);
}
