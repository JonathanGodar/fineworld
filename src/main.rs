use std::time::Duration;

use bevy::{
    prelude::*,
    app::{ScheduleRunnerSettings, ScheduleRunnerPlugin, AppExit}, render::camera::RenderTarget, time::FixedTimestep,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .init_resource::<CursorPosition>()
        .insert_resource(ClearColor(Color::rgb(1., 0., 0.)))
        .add_startup_system(setup)
        .add_system_to_stage(CoreStage::PreUpdate, update_cursor_position_system)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(paddle_ai_system.before(check_for_collisions))
                .with_system(apply_velocity.before(check_for_collisions))
                .with_system(check_for_collisions)
            )
        .add_system(controlled_paddle_movement_system)
        .add_system(escape_system)
        .run();
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Controlled;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Ball; 

#[derive(Component)]
struct Collider;

#[derive(Bundle)]
struct BallBundle {
    ball: Ball,
    velocity: Velocity,
    sprite_bundle: SpriteBundle,
    collider: Collider,
}

#[derive(Component)]
struct AI;

#[derive(Resource, Default, Debug)]
struct CursorPosition(Vec2);

fn create_sprite(color: Color, pos: Vec3, size: Vec2) -> SpriteBundle {
    SpriteBundle {
        transform: Transform { translation: pos, scale: Vec3::new(size.x, size.y, 1.), ..default()},
        sprite: Sprite  {
            color,
            ..default()
        },
        ..default()
    }
}

// Paddle constants
const PADDLE_WIDTH: f32 = 20.;
const PADDLE_HEIGHT: f32 = 80.;

const LEFT_PADDLE_X_POS: f32 = -300.0;
const RIGHT_PADDLE_X_POS: f32 = -LEFT_PADDLE_X_POS;
const PADDLE_COLOR: Color = Color::DARK_GREEN;

// Ball constants
const BALL_COLOR: Color = Color::ORANGE;
const BALL_WIDTH: f32 = 20.;
const BALL_HEIGHT: f32 = 20.;

// General constants
const TIME_STEP: f32 = 0.01666;


fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle::default(),
        MainCamera,
    ));

    commands.spawn((
            create_sprite(PADDLE_COLOR, Vec3::new(LEFT_PADDLE_X_POS, 0., 0.), Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT)),
            Paddle,
            Controlled,
            Collider,
    ));

    commands.spawn((
            create_sprite(PADDLE_COLOR, Vec3::new(RIGHT_PADDLE_X_POS, 0., 0.), Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT)),
            Paddle,
            AI,
            Collider
    ));

    commands.spawn(BallBundle {
        ball: Ball,
        velocity: Velocity(Vec2::new(100., 100.)),
        sprite_bundle: create_sprite(BALL_COLOR, Vec3::ZERO, Vec2::new(BALL_WIDTH, BALL_HEIGHT)),
        collider: Collider,
    });
}


fn apply_velocity(mut q: Query<(&Velocity, &mut Transform)>){
    for (vel, mut transform) in q.iter_mut() {
        transform.translation.x += vel.x * TIME_STEP;
        transform.translation.y += vel.y * TIME_STEP;
    }
}


// fn update_cursor_position_system(wnds: Res<Windows>, q_camera: Query<(&Camera, &GlobalTransform)>, mut cursor_pos: ResMut<CursorPosition>) {
fn update_cursor_position_system(wnds: Res<Windows>, q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>, mut cursor_pos: ResMut<CursorPosition>) {
    let (camera, camera_transform) = q_camera.single();

    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get(id).unwrap()
    } else {
        wnds.get_primary().unwrap()
    };

    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = wnd.cursor_position() {
        // get the size of the window
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

        // matrix for undoing the projection and camera transform
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();

        // use it to convert ndc to world-space coordinates
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

        // reduce it to a 2D value
        let world_pos: Vec2 = world_pos.truncate();

        cursor_pos.0 = world_pos;
    }

}


fn check_for_collisions(
    mut ball_query: Query<(&Transform, &mut Velocity), With<Ball>>,
    colliders_query: Query<(&Transform), (With<Collider>, Without<Ball>)>
){
    let (ball_transform, mut ball_vel)  = ball_query.single_mut();

    {
        let mut reflect_x = false;
        let mut reflect_y = false;
        if ball_transform.translation.x < -400. || ball_transform.translation.x > 400. {
            reflect_x = true;
        }

        if ball_transform.translation.y < -400. || ball_transform.translation.y > 400. {
            reflect_y = true;
        }

        for (&transform) in colliders_query.iter() {
        }



        if reflect_x { ball_vel.x *= -1. };
        if reflect_y { ball_vel.y *= -1. };
    }
}

fn paddle_ai_system(mut paddles: Query<&mut Transform, (With<AI>, With<Paddle>)>, ball: Query<&Transform, (With<Ball>, Without<Paddle>)>) {
 let ball_pos = ball.single().translation;
 for mut paddle_pos in paddles.iter_mut() {
     paddle_pos.translation.y = ball_pos.y;
 }
}


fn controlled_paddle_movement_system(mut q: Query<&mut Transform, (With<Paddle>, With<Controlled>)>, cursor_pos: Res<CursorPosition>) {
    q.single_mut().translation.y = cursor_pos.0.y; 
}

fn escape_system(mut exit: EventWriter<AppExit>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}
