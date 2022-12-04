use std::time::Duration;

use bevy::{
    prelude::*,
    app::{ScheduleRunnerSettings, ScheduleRunnerPlugin, AppExit}, render::camera::RenderTarget,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .init_resource::<CursorPosition>()
        .insert_resource(ClearColor(Color::rgb(1., 0., 0.)))
        .add_startup_system(setup)
        .add_system_to_stage(CoreStage::PreUpdate, update_cursor_position_system)
        .add_system(controlled_paddle_movement_system)
        // .add_system(update_cursor_position_system)
        .add_system(escape_system)
        .run();
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Controlled;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Ball; 

#[derive(Bundle)]
struct BallBundle {
    ball: Ball,
    velocity: Velocity,
    sprite_bundle: SpriteBundle,
}


#[derive(Resource, Default, Debug)]
struct CursorPosition(Vec2);

// #[derive(Bundle)]
// struct PaddleBundle {
//    paddle: Paddle,
//    sprite_bundle: SpriteBundle,
// }


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

const LEFT_PADDLE_X_POS: f32 = -20.0;
const PADDLE_COLOR: Color = Color::DARK_GREEN;

// Ball constants
const BALL_COLOR: Color = Color::ORANGE;
const BALL_WIDTH: f32 = 20.;
const BALL_HEIGHT: f32 = 20.;

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle::default(),
        MainCamera,
    ));

    commands.spawn((
            create_sprite(PADDLE_COLOR, Vec3::new(LEFT_PADDLE_X_POS, 0., 0.), Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT)),
            Paddle,
            Controlled,
    ));

    commands.spawn(BallBundle {
        ball: Ball,
        velocity: Velocity(Vec2::new(0.1, 0.1)),
        sprite_bundle: create_sprite(BALL_COLOR, Vec3::ZERO, Vec2::new(BALL_WIDTH, BALL_HEIGHT)),
    });
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

fn controlled_paddle_movement_system(mut q: Query<&mut Transform, (With<Paddle>, With<Controlled>)>, cursor_pos: Res<CursorPosition>) {
    q.single_mut().translation.y = cursor_pos.0.y; 
}

fn escape_system(mut exit: EventWriter<AppExit>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}
