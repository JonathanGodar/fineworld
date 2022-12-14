mod camera;

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
use camera::failed_camera::{FailedCameraBundle, FailedCameraPlugin};

#[derive(Hash, Clone, Debug, Eq, PartialEq)]
enum AppState {
    AssetValidation,
    PreGame,
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
    // Custom Plugins
    .add_plugin(FailedCameraPlugin::default())
    // Custom resources
    .init_resource::<BlockTextureHandles>()
    .init_resource::<UvMappings>()
    // Startup Systems
    .add_startup_system(setup_camera)
    .add_startup_system(load_textures)
    .add_startup_system(setup_world)
    .add_state(AppState::AssetValidation)
    .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
    .add_system_set(SystemSet::on_update(AppState::AssetValidation).with_system(validate_textures))
    .add_system_set(SystemSet::on_enter(AppState::Game).with_system(generate_chunk))
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

type UVs = [[f32; 2]; 4];
#[derive(Resource, Deref, DerefMut, Default, Debug)]
struct UvMappings(HashMap<BlockType, (UVs, UVs, UVs)>);

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

    commands
        .spawn(FailedCameraBundle {
            camera_bundle: Camera3dBundle {
                transform: Transform::from_xyz(0., 5., 10.).looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            failed_camera: camera::failed_camera::FailedCamera,
        })
        .insert(MainCamera);
}

const CHUNK_SIZE: IVec3 = IVec3::new(16, 32, 16);

#[derive(Component, Default)]
struct Chunk {
    chunk_coords: IVec3,
    world_seed: u64,
    blocks: [[[BlockType; CHUNK_SIZE.z as usize]; CHUNK_SIZE.y as usize]; CHUNK_SIZE.x as usize],
}

impl Chunk {
    fn generate_terrain(mut self) -> Self {
        for (pos, block) in self.iter_blocks_mut() {
            if pos.y <= 5 {
                *block = BlockType::Placeholder
            };
        }

        self
    }

    fn iter_blocks_mut(&mut self) -> impl Iterator<Item = (IVec3, &mut BlockType)> {
        self.blocks
            .iter_mut()
            .enumerate()
            .flat_map(move |(x, yz_slice)| {
                yz_slice
                    .iter_mut()
                    .enumerate()
                    .flat_map(move |(y, z_strip)| {
                        z_strip.iter_mut().enumerate().map(move |(z, block)| {
                            (IVec3::new(x as i32, y as i32, z as i32), block)
                        })
                    })
            })
    }

    fn iter_blocks(&self) -> impl Iterator<Item = (IVec3, &BlockType)> {
        self.blocks
            .iter()
            .enumerate()
            .flat_map(move |(x, yz_slice)| {
                yz_slice.iter().enumerate().flat_map(move |(y, z_strip)| {
                    z_strip
                        .iter()
                        .enumerate()
                        .map(move |(z, block)| (IVec3::new(x as i32, y as i32, z as i32), block))
                })
            })
    }

    fn get_block(&self, pos: IVec3) -> Option<&BlockType> {
        if !Chunk::is_within_bounds(pos) {
            return None;
        }

        return Some(&self.blocks[pos.x as usize][pos.y as usize][pos.z as usize]);
    }

    fn get_block_mut(&mut self, pos: IVec3) -> Option<&mut BlockType> {
        if !Chunk::is_within_bounds(pos) {
            return None;
        }

        return Some(&mut self.blocks[pos.x as usize][pos.y as usize][pos.z as usize]);
    }

    #[inline]
    fn is_within_bounds(pos: IVec3) -> bool {
        return pos.x >= 0
            && pos.x < CHUNK_SIZE.x
            && pos.y >= 0
            && pos.y < CHUNK_SIZE.y
            && pos.z >= 0
            && pos.z < CHUNK_SIZE.z;
    }

    fn construct_mesh(&self, uv_mappings: &UvMappings) -> Mesh {
        let mut indicies = Vec::new();
        let mut vertecies = Vec::new();

        let mut uvs: Vec<[f32; 2]> = Vec::new();

        for (pos, block) in self.iter_blocks() {
            if self.get_block(pos) == Some(&BlockType::Air) {
                continue;
            }

            let top_visible = self
                .get_block(pos + IVec3::Y)
                .map_or(true, |block| block.is_transparent());

            let front_visible = self
                .get_block(pos + IVec3::Z)
                .map_or(true, |block| block.is_transparent());

            let right_visible = self
                .get_block(pos + IVec3::X)
                .map_or(true, |block| block.is_transparent());

            let back_visible = self
                .get_block(pos + IVec3::NEG_Z)
                .map_or(true, |block| block.is_transparent());

            let left_visible = self
                .get_block(pos + IVec3::NEG_X)
                .map_or(true, |block| block.is_transparent());

            let bottom_visible = self
                .get_block(pos + IVec3::NEG_Y)
                .map_or(true, |block| block.is_transparent());

            if !top_visible
                && !front_visible
                && !right_visible
                && !back_visible
                && !left_visible
                && !bottom_visible
            {
                continue;
            }

            let fpos = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
            // vertecies.extend_from_slice(&[
            //     [fpos.x, fpos.y, fpos.z],
            //     [fpos.x + 1., fpos.y, fpos.z],
            //     [fpos.x + 1., fpos.y, fpos.z + 1.],
            //     [fpos.x, fpos.y, fpos.z + 1.],
            //     [fpos.x, fpos.y + 1., fpos.z],
            //     [fpos.x + 1., fpos.y + 1., fpos.z],
            //     [fpos.x + 1., fpos.y + 1., fpos.z + 1.],
            //     [fpos.x, fpos.y + 1., fpos.z + 1.],
            // ]);

            if top_visible {
                let vertex_offset = vertecies.len() as u32;
                vertecies.extend_from_slice(&[
                    [fpos.x, fpos.y + 1., fpos.z],
                    [fpos.x + 1., fpos.y + 1., fpos.z],
                    [fpos.x + 1., fpos.y + 1., fpos.z + 1.],
                    [fpos.x, fpos.y + 1., fpos.z + 1.],
                ]);

                indicies.extend_from_slice(&[
                    vertex_offset + 3,
                    vertex_offset + 1,
                    vertex_offset + 0,
                    // Second triangle
                    vertex_offset + 3,
                    vertex_offset + 2,
                    vertex_offset + 1,
                ]);

                let uv = &uv_mappings.get(block).expect("Texture not found").0;
                uvs.extend_from_slice(uv);
            };

            if front_visible {
                let vertex_offset = vertecies.len() as u32;
                vertecies.extend_from_slice(&[
                    [fpos.x, fpos.y + 1., fpos.z + 1.],
                    [fpos.x + 1., fpos.y + 1., fpos.z + 1.],
                    [fpos.x + 1., fpos.y, fpos.z + 1.],
                    [fpos.x, fpos.y, fpos.z + 1.],
                ]);

                indicies.extend_from_slice(&[
                    vertex_offset + 3,
                    vertex_offset + 1,
                    vertex_offset + 0,
                    // Second triangle
                    vertex_offset + 3,
                    vertex_offset + 2,
                    vertex_offset + 1,
                ]);

                let uv = &uv_mappings.get(block).expect("Texture not found").1;
                uvs.extend_from_slice(uv);
            }

            if right_visible {
                let vertex_offset = vertecies.len() as u32;
                vertecies.extend_from_slice(&[
                    [fpos.x + 1., fpos.y + 1., fpos.z + 1.],
                    [fpos.x + 1., fpos.y + 1., fpos.z],
                    [fpos.x + 1., fpos.y, fpos.z],
                    [fpos.x + 1., fpos.y, fpos.z + 1.],
                ]);

                indicies.extend_from_slice(&[
                    vertex_offset + 3,
                    vertex_offset + 1,
                    vertex_offset + 0,
                    // Second triangle
                    vertex_offset + 3,
                    vertex_offset + 2,
                    vertex_offset + 1,
                ]);

                let uv = &uv_mappings.get(block).expect("Texture not found").1;
                uvs.extend_from_slice(uv);
            }

            if back_visible {
                let vertex_offset = vertecies.len() as u32;
                vertecies.extend_from_slice(&[
                    [fpos.x + 1., fpos.y + 1., fpos.z],
                    [fpos.x, fpos.y + 1., fpos.z],
                    [fpos.x, fpos.y, fpos.z],
                    [fpos.x + 1., fpos.y, fpos.z],
                ]);

                indicies.extend_from_slice(&[
                    vertex_offset + 3,
                    vertex_offset + 1,
                    vertex_offset + 0,
                    // Second triangle
                    vertex_offset + 3,
                    vertex_offset + 2,
                    vertex_offset + 1,
                ]);

                let uv = &uv_mappings.get(block).expect("Texture not found").1;
                uvs.extend_from_slice(uv);
            }

            if left_visible {
                let vertex_offset = vertecies.len() as u32;
                vertecies.extend_from_slice(&[
                    [fpos.x, fpos.y + 1., fpos.z],
                    [fpos.x, fpos.y + 1., fpos.z + 1.],
                    [fpos.x, fpos.y, fpos.z + 1.],
                    [fpos.x, fpos.y, fpos.z],
                ]);

                indicies.extend_from_slice(&[
                    vertex_offset + 3,
                    vertex_offset + 1,
                    vertex_offset + 0,
                    // Second triangle
                    vertex_offset + 3,
                    vertex_offset + 2,
                    vertex_offset + 1,
                ]);

                let uv = &uv_mappings.get(block).expect("Texture not found").1;
                uvs.extend_from_slice(uv);
            }

            if bottom_visible {
                let vertex_offset = vertecies.len() as u32;
                vertecies.extend_from_slice(&[
                    [fpos.x, fpos.y, fpos.z + 1.],
                    [fpos.x + 1., fpos.y, fpos.z + 1.],
                    [fpos.x + 1., fpos.y, fpos.z],
                    [fpos.x, fpos.y, fpos.z],
                ]);

                indicies.extend_from_slice(&[
                    vertex_offset + 3,
                    vertex_offset + 1,
                    vertex_offset + 0,
                    // Second triangle
                    vertex_offset + 3,
                    vertex_offset + 2,
                    vertex_offset + 1,
                ]);

                let uv = &uv_mappings.get(block).expect("Texture not found").2;
                uvs.extend_from_slice(uv);
            }

            //             if front_visible {
            //                 // Front faces
            //                 indicies.extend_from_slice(&[
            //                     vertex_offset + 3,
            //                     vertex_offset + 6,
            //                     vertex_offset + 7,
            //                     // Second triangle
            //                     vertex_offset + 3,
            //                     vertex_offset + 2,
            //                     vertex_offset + 6,
            //                 ]);
            //             }

            //             if right_visible {
            //                 // Right faces
            //                 indicies.extend_from_slice(&[
            //                     vertex_offset + 5,
            //                     vertex_offset + 6,
            //                     vertex_offset + 2,
            //                     // Second triangle
            //                     vertex_offset + 1,
            //                     vertex_offset + 5,
            //                     vertex_offset + 2,
            //                 ]);
            //             }

            //             if back_visible {
            //                 // Back faces
            //                 indicies.extend_from_slice(&[
            //                     vertex_offset + 1,
            //                     vertex_offset + 0,
            //                     vertex_offset + 5,
            //                     // Second triangle
            //                     vertex_offset + 0,
            //                     vertex_offset + 4,
            //                     vertex_offset + 5,
            //                 ]);
            //             }

            //             if left_visible {
            //                 // Left faces
            //                 indicies.extend_from_slice(&[
            //                     vertex_offset + 7,
            //                     vertex_offset + 4,
            //                     vertex_offset + 0,
            //                     // Second triangle
            //                     vertex_offset + 0,
            //                     vertex_offset + 3,
            //                     vertex_offset + 7,
            //                 ]);
            //             }

            //             if bottom_visible {
            //                 // Bottom faces
            //                 indicies.extend_from_slice(&[
            //                     vertex_offset + 3,
            //                     vertex_offset,
            //                     vertex_offset + 1,
            //                     // Second triangle
            //                     vertex_offset + 2,
            //                     vertex_offset + 3,
            //                     vertex_offset + 1,
            //                 ]);
            //             }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        mesh.set_indices(Some(Indices::U32(indicies)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertecies);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh
    }
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

#[derive(Default, PartialEq, Debug, Reflect, Eq, PartialOrd, Ord, Hash)]
enum BlockType {
    #[default]
    Air,
    Grass,
    Stone,
    Placeholder,
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
    // println!(
    //     "{:#?}",
    //     asset_server.get_load_state(block_textures.0.clone())
    // );

    println!("Generating chunk");
    let chunk_coords = IVec3::ZERO;
    let world_seed = 0u64;

    let chunk = Chunk {
        world_seed,
        ..default()
    }
    .generate_terrain();

    let mesh = chunk.construct_mesh(&uv_mappings);

    let atlas_image_handle = texture_atlases.get(&atlas_handle).unwrap().texture.clone();

    let material = materials.add(StandardMaterial {
        base_color: Color::ORANGE,
        base_color_texture: Some(atlas_image_handle),
        unlit: true,
        ..default()
    });

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

    commands.spawn(chunk_bundle).insert(Wireframe);
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
