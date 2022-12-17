use std::time::Duration;

use bevy::{
    prelude::*,
    app::{ScheduleRunnerSettings, ScheduleRunnerPlugin, AppExit}, render::camera::RenderTarget, time::FixedTimestep, sprite::collide_aabb::{collide, Collision},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}
};



fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
    	.add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
            )
        .add_system(escape_system)
        .run();
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct ScreenBound;


fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera2dBundle{
        ..Default::default()
    });

    commands.insert_resource(ClearColor(Color::BLACK));

    commands.spawn((
        Paddle,
        SpriteBundle {
            sprite: Sprite {
                color: Color::GRAY,
                custom_size: Some(Vec2::new(200.0, 200.0)),
                ..default()
            },
            transform: Transform::from_xyz(0., 0., 1.),
            ..default()
        }
    ));

    // commands.spawn(PointLightBundle {
    //     transform: Transform::from_xyz(10., 10., 10.),
    //     ..default()
    // });

    // commands.spawn((PbrBundle {
    //     mesh: meshes.add(shape::Plane {
    //         size: 100f32,
    //         ..default()
    //     }.into()),
    //     material: materials.add(Color::ORANGE.into()),
    //     ..default()
    // }));

    // commands.spawn((Camera3dBundle {
    //     transform: Transform::from_xyz(5., 105., 100.).looking_at(Vec3::ZERO, Vec3::Y),
    //     ..default()
    // }, MainCamera));


    // commands.spawn();
}

fn escape_system(mut exit: EventWriter<AppExit>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}
