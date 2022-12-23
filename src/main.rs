mod block;
mod camera;
mod chunk;
mod player;

use std::collections::HashMap;

use bevy::{
    asset::LoadState,
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    reflect::{DynamicEnum, DynamicVariant},
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    time::FixedTimestep,
    window::{close_on_esc, CursorGrabMode},
};
use bevy_rapier3d::prelude::*;
use block::UvMappings;
use camera::failed_camera::{camera_pitch_system, FailedCameraBundle, FailedCameraPlugin};
use chunk::Chunk;
use player::{character_velocity_system, gravity_system, player_movement_system, setup_player};

use crate::block::{BlockType, UVs};

#[derive(Hash, Clone, Debug, Eq, PartialEq)]
enum AppState {
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
    // Bevy resources
    .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.8)))
    // External Plugins
    .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
    .add_plugin(RapierDebugRenderPlugin::default())
    // Custom Plugins
    .add_plugin(FailedCameraPlugin::default())
    // Custom resources
    .init_resource::<BlockTextureHandles>()
    .init_resource::<UvMappings>()
    // Startup Systems
    .add_startup_system(setup_camera)
    .add_startup_system(load_textures)
    .add_startup_system(setup_world)
    .add_startup_system(setup_player)
    .add_state(AppState::AssetValidation)
    .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
    .add_system_set(SystemSet::on_update(AppState::AssetValidation).with_system(validate_textures))
    .add_system_set(SystemSet::on_enter(AppState::Game).with_system(generate_chunk))
    .add_system_set(
        SystemSet::on_update(AppState::Game)
            .label("wanted_movements")
            .before("movement")
            .with_system(player_movement_system)
            .with_system(gravity_system)
            .with_system(camera_pitch_system),
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

#[derive(Deref, DerefMut, Resource, Default)]
struct BlockTextureHandles(Vec<HandleUntyped>);

#[derive(Resource, DerefMut, Deref, Clone)]
struct BlockAtlasHandle(Handle<TextureAtlas>);

// #[derive(Deref, DerefMut)]
// pub struct BlockTextureAtlas(TextureAtlas);
// #[derive(Deref, DerefMut, Resource, Default)]
// struct BlockTextureAtlas(TextureAtlas);

fn construct_atlas(
    mut commands: Commands,
    block_texture_handles: Res<BlockTextureHandles>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Image>>,
    mut uv_mappings: ResMut<UvMappings>,
) {
    let mut texture_atlas_builder = TextureAtlasBuilder::default();
    for handle in block_texture_handles.iter() {
        let handle = handle.typed_weak();
        let Some(texture) = textures.get(&handle) else {
            panic!("{:?} did not resolve to an `Image` asset.", asset_server.get_handle_path(handle));
        };

        texture_atlas_builder.add_texture(handle, texture);
    }

    let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();

    let mut textures: HashMap<String, (UVs, UVs, UVs)> = HashMap::new();
    for handle in block_texture_handles.iter() {
        if let Some(handle_path) = asset_server.get_handle_path(handle) {
            let handle_path = handle_path.path().strip_prefix("textures/blocks");
            if handle_path.is_err() {
                warn!("something funky is goinng on in the texture loading");
                continue;
            }
            let handle_path = handle_path.unwrap();

            let texture_idx = texture_atlas
                .get_texture_index(&handle.typed_weak())
                .unwrap();

            let texture_uvs = {
                let atlas_size = texture_atlas.size;
                let image_rect = texture_atlas.textures[texture_idx];

                let top_left = (image_rect.min / atlas_size).to_array();
                let top_right =
                    ((image_rect.min + Vec2::new(image_rect.width(), 0.)) / atlas_size).to_array();

                let bottom_right = (image_rect.max / atlas_size).to_array();
                let bottom_left =
                    ((image_rect.min + Vec2::new(0., image_rect.height())) / atlas_size).to_array();

                // let a
                [
                    top_left,
                    top_right,
                    bottom_right,
                    bottom_left,
                    // bottom_left,
                    // top_right,
                    // top_left,
                    // bottom_left,
                    // bottom_right,
                    // top_right,
                ]
            };

            let texture_position = handle_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let block_name = handle_path
                .components()
                .into_iter()
                .next()
                .unwrap()
                .as_os_str()
                .to_string_lossy()
                .to_string();

            if texture_position.as_str() == "texture" {
                textures.insert(
                    block_name,
                    (texture_uvs.clone(), texture_uvs.clone(), texture_uvs),
                );
            } else {
                const NULL_UV: [[f32; 2]; 4] = [[0., 0.], [0., 0.], [0., 0.], [0., 0.]];

                textures
                    .entry(block_name.clone())
                    .or_insert((NULL_UV, NULL_UV, NULL_UV));
                println!("tex_pos {:?}, block {:?}", texture_position, block_name);
                match texture_position.as_str() {
                    "top" => {
                        let (top, _, _) = textures.get_mut(&block_name).unwrap();
                        *top = texture_uvs;
                    }
                    "side" => {
                        let (_, side, _) = textures.get_mut(&block_name).unwrap();
                        *side = texture_uvs;
                    }
                    "bottom" => {
                        let (_, _, bottom) = textures.get_mut(&block_name).unwrap();
                        *bottom = texture_uvs;
                    }
                    _ => {
                        panic!("Some bad input to the texture loading")
                    }
                }
            }
        }
    }

    {
        let placeholder = textures.remove("Placeholder");
        uv_mappings
            .0
            .insert(BlockType::Placeholder, placeholder.unwrap());
    }

    // let reflect = BlockType::as_reflect(&BlockType::Air).;
    for entry in textures {
        let mut curr_type = BlockType::Air;
        let dynamic_enum = DynamicEnum::new(
            Reflect::type_name(&BlockType::Air),
            &entry.0,
            DynamicVariant::Unit,
        );

        curr_type.apply(&dynamic_enum);

        (*uv_mappings).insert(curr_type, entry.1);
    }

    println!("{:#?}", *uv_mappings);

    commands.insert_resource(BlockAtlasHandle(texture_atlases.add(texture_atlas)));
    commands.remove_resource::<BlockTextureHandles>();
}

fn validate_textures(
    mut state: ResMut<State<AppState>>,
    asset_server: Res<AssetServer>,
    block_texture_handles: Res<BlockTextureHandles>,
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Image>>,
    uv_mappings: ResMut<UvMappings>,
) {
    // println!("IN Validate textures");

    if let LoadState::Loaded =
        asset_server.get_group_load_state(block_texture_handles.iter().map(|h| h.id))
    {
        // println!("Changing state to pregame");
        state.set(AppState::Game).unwrap();
        construct_atlas(
            commands,
            block_texture_handles,
            asset_server,
            texture_atlases,
            textures,
            uv_mappings,
        );
    }
}

fn load_textures(
    asset_server: Res<AssetServer>,
    mut block_texture_handles: ResMut<BlockTextureHandles>,
) {
    block_texture_handles.0 = asset_server.load_folder("./textures/blocks").unwrap();
}

#[derive(Component)]
struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(-5., 5., 5.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        directional_light: DirectionalLight {
            color: Color::WHITE,
            illuminance: 50_000.,
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}

#[derive(Deref, DerefMut)]
struct BlocksAtlas(TextureAtlas);

// fn load_textures(mut commands: Commands, asset_server: Res<AssetServer>, block_atlas: ResMut<) {
//     let loaded = asset_server.load_folder("./textures/blocks").unwrap();

//     for

//     // asset_server.get_handle_path
//     // asset_server.add_loader();
// }

enum BlockFace {
    Top,
    Front,
    Right,
    Back,
    Left,
    Bottom,
}

struct BlockData {
    is_transparent: bool,
}

impl BlockType {
    fn is_transparent(&self) -> bool {
        return *self == BlockType::Air;
    }

    fn get_uvs(&self) -> [[[f32; 2]; 4]; 6] {
        const texture_count: i32 = 3;
        const texture_size: f32 = 1. / texture_count as f32;
        match *self {
            _ => [
                [
                    [texture_size * 2., 0.],
                    [texture_size * 2., texture_size],
                    [texture_size * 3., texture_size],
                    [texture_size * 3., 0.],
                ],
                [
                    [texture_size * 2., 0.],
                    [texture_size * 2., texture_size],
                    [texture_size * 3., texture_size],
                    [texture_size * 3., 0.],
                ],
                [
                    [texture_size * 2., 0.],
                    [texture_size * 2., texture_size],
                    [texture_size * 3., texture_size],
                    [texture_size * 3., 0.],
                ],
                [
                    [texture_size * 2., 0.],
                    [texture_size * 2., texture_size],
                    [texture_size * 3., texture_size],
                    [texture_size * 3., 0.],
                ],
                [
                    [texture_size * 2., 0.],
                    [texture_size * 2., texture_size],
                    [texture_size * 3., texture_size],
                    [texture_size * 3., 0.],
                ],
                [
                    [texture_size * 2., 0.],
                    [texture_size * 2., texture_size],
                    [texture_size * 3., texture_size],
                    [texture_size * 3., 0.],
                ],
            ],
        }
    }
    // fn get_data(&self) -> BlockData {
    //     BlockData {
    //         is_transparent: *self == BlockType::Air,
    //     }
    // }
}

#[derive(Bundle)]
struct ChunkBundle {
    chunk: Chunk,
    pbr: PbrBundle,
}

fn generate_chunk(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    uv_mappings: Res<UvMappings>,
    asset_server: Res<AssetServer>,
    atlas_handle: Res<BlockAtlasHandle>,
    texture_atlases: Res<Assets<TextureAtlas>>,
) {
    println!("Generating chunk");
    let chunk_coords = IVec3::ZERO;
    let world_seed = 0u64;

    let chunk = Chunk {
        world_seed,
        ..default()
    }
    .generate_terrain();

    let (mesh, collider) = chunk.construct_mesh(&uv_mappings);

    let atlas_image_handle = texture_atlases.get(&atlas_handle).unwrap().texture.clone();

    let material = materials.add(StandardMaterial {
        base_color: Color::ORANGE,
        base_color_texture: Some(atlas_image_handle),
        unlit: true,
        ..default()
    });

    // let collider = Collider::trimesh(mesh.)

    commands.spawn(PbrBundle {
        material: material.clone(),
        mesh: meshes.add(shape::Quad::new(Vec2::new(10., 10.)).into()),
        ..Default::default()
    });

    let pbr_bundle = PbrBundle {
        material,
        mesh: meshes.add(mesh),
        ..default()
    };

    let chunk_bundle = ChunkBundle {
        pbr: pbr_bundle,
        chunk: Chunk {
            chunk_coords,
            world_seed,
            ..default()
        },
    };

    commands
        .spawn(chunk_bundle)
        .insert(collider)
        .insert(RigidBody::Fixed); //.insert(Wireframe);
}

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

    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(shape::Plane { size: 20. }.into()),
    //     material,
    //     transform: Transform::from_xyz(0., -5., 0.),
    //     ..Default::default()
    // });
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
