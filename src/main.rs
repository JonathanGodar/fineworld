mod block;
mod camera;
mod chunk;
mod game_world;
mod player;



use bevy::{
    pbr::wireframe::{WireframePlugin},
    prelude::*,
    time::FixedTimestep,
    window::{close_on_esc, CursorGrabMode}, diagnostic::{LogDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
};
use bevy_rapier3d::prelude::*;
use block::{
    textures::textures::{load_textures, validate_textures},
    BlockTextureHandles, UvMappingsRes,
};
use camera::failed_camera::{camera_pitch_system, FailedCameraBundle, FailedCameraPlugin};
use chunk::{
    chunk_load_system, chunk_unload_system, Chunk,
    LoadedChunks, GeneratingChunks, queue_mesh_generation_system, handle_generated_chunks_system, 
};
use game_world::WorldSeed;
use player::{character_velocity_system, gravity_system, player_movement_system, setup_player};

use crate::block::{BlockType, UVs};

#[derive(Hash, Clone, Debug, Eq, PartialEq)]
pub enum AppState {
    AssetValidation,
    Game,
}

const TIME_STEP: f32 = 0.016;
fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                window: WindowDescriptor {
                    monitor: MonitorSelection::Index(1),
                    ..default()
                },
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    )
    // Bevy Plugins
    .add_plugin(WireframePlugin::default())
    .add_plugin(LogDiagnosticsPlugin::default())
    .add_plugin(FrameTimeDiagnosticsPlugin::default())
    // Bevy resources
    .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.8)))

    // External Plugins
    .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    // .add_plugin(RapierDebugRenderPlugin::default())
    // Custom Plugins
    // .add_plugin(FailedCameraPlugin::default())
    // Custom resources
    .init_resource::<BlockTextureHandles>()
    .init_resource::<LoadedChunks>()
    .init_resource::<GeneratingChunks>()
    .init_resource::<WorldSeed>()
    // Startup Systems
    .add_startup_system(load_textures)
    .add_startup_system(setup_world)
    .add_startup_system(setup_player)
    .add_state(AppState::AssetValidation)
    .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
    .add_system_set(SystemSet::on_update(AppState::AssetValidation).with_system(validate_textures))
    // .add_system_set(SystemSet::on_enter(AppState::Game).with_system(chunk_load_enqueue_system))
    .add_system_set(
        SystemSet::on_update(AppState::Game)
            .label("wanted_movements")
            .before("movement")
            .with_system(player_movement_system)
            .with_system(gravity_system)
            .with_system(camera_pitch_system)
            .with_system(chunk_load_system)
            // .with_system(process_chunk_tasks_system)
            .with_system(handle_generated_chunks_system)
            .with_system(queue_mesh_generation_system)
            .with_system(chunk_unload_system),
    )
    .add_system_set(
        SystemSet::on_update(AppState::Game)
            .label("movement")
            .with_system(character_velocity_system),
    )
    .add_system(close_on_esc)
    .add_system(cursor_lock_system)
    .run();
}



#[derive(Component)]
struct MainCamera;

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let material = materials.add(StandardMaterial::from(Color::GREEN));
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Box::new(2., 2., 5.).into()),
        material: material.clone(),
        ..Default::default()
    });
}

fn cursor_lock_system(mut windows: ResMut<Windows>, keys: Res<Input<KeyCode>>) {
    fn toggle_mouse_lock(window: &mut Window) {
        match window.cursor_grab_mode() {
            CursorGrabMode::Locked => {
                window.set_cursor_grab_mode(CursorGrabMode::None);
                window.set_cursor_visibility(true);
            }
            _ => {
                window.set_cursor_grab_mode(CursorGrabMode::Locked);
                window.set_cursor_visibility(false);
            }
        }
    }

    let window = windows.primary_mut();

    if keys.pressed(KeyCode::Return) {
        toggle_mouse_lock(window);
    }

    if !window.is_focused() && !window.cursor_visible() {
        toggle_mouse_lock(window);
    }
}
