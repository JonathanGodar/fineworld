mod block;
mod camera;
mod chunk;
mod player;

use bitflags::bitflags;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::wireframe::WireframePlugin,
    prelude::*,
    time::FixedTimestep,
    window::{close_on_esc, CursorGrabMode},
};
use bevy_rapier3d::prelude::*;
use block::{
    textures::textures::{load_textures, validate_textures},
    BlockTextureHandles, UvMappingsRes,
};
use camera::failed_camera::{camera_pitch_system, FailedCameraBundle, FailedCameraPlugin};
use chunk::chunk_manager::{
    chunk_load_system, chunk_unload_system, handle_chunk_updates, handle_generated_chunks_system,
    mesh_generation_system, ChunkManager, GeneratingChunks, LoadedChunks, WorldSeed,
};
use player::{
    block_break_system, character_velocity_system, gravity_system, player_movement_system,
    setup_player_system,
};

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
    // .add_plugin(FrameTimeDiagnosticsPlugin::default())
    // Bevy resources
    .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.8)))
    // External Plugins
    .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    // .add_plugin(RapierDebugRenderPlugin::default())
    // Custom Plugins
    // .add_plugin(FailedCameraPlugin::default())
    // Custom events
    // Custom resources
    .init_resource::<BlockTextureHandles>()
    .init_resource::<LoadedChunks>()
    .init_resource::<GeneratingChunks>()
    .init_resource::<WorldSeed>()
    .init_resource::<ChunkManager>()
    // Startup Systems
    .add_startup_system(load_textures)
    .add_startup_system(setup_world)
    .add_startup_system(setup_player_system)
    .add_state(AppState::AssetValidation)
    .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
    .add_system_set(SystemSet::on_update(AppState::AssetValidation).with_system(validate_textures))
    // .add_system_set(
    // SystemSet::on_update(AppState::Game)
    //     .label()
    // )
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
            .with_system(chunk_unload_system)
            .with_system(block_break_system)
            .with_system(handle_chunk_updates.after(block_break_system))
            .with_system(mesh_generation_system.after(handle_chunk_updates)),
    )
    // .add_system()
    .add_system_set(
        SystemSet::on_update(AppState::Game)
            .label("movement")
            .with_system(character_velocity_system),
    )
    .add_system(close_on_esc)
    .add_system(cursor_lock_system)
    .run();
}

bitflags! {
    struct CollisionLayers: u32 {
        const PLAYER = 0b00000001;
        const TERRAIN = 0b00000010;
    }
}

#[derive(Component)]
pub struct MainCamera;

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
