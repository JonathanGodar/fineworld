mod camera;
use std::time::Duration;

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin, ScheduleRunnerSettings},
    input::mouse::MouseMotion,
    prelude::*,
    render::{camera::RenderTarget, render_resource::PrimitiveTopology},
    sprite::collide_aabb::{collide, Collision},
    time::FixedTimestep,
    window::{close_on_esc, CursorGrabMode},
};
use camera::failed_camera::{FailedCameraBundle, FailedCameraPlugin};

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
    .add_plugin(FailedCameraPlugin::default())
    .insert_resource(ClearColor(Color::rgb(0.2, 0.2, 0.8)))
    .add_startup_system(setup_camera)
    .add_startup_system(setup_world)
    .add_startup_system(generate_chunk)
    .add_system_set(SystemSet::new().with_run_criteria(FixedTimestep::step(TIME_STEP as f64)))
    .add_system(close_on_esc)
    .add_system(cursor_lock_system)
    .run();
}

#[derive(Component)]
struct MainCamera;

fn setup_camera(mut commands: Commands, mut windows: ResMut<Windows>) {
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(-5., 5., 5.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        directional_light: DirectionalLight {
            color: Color::ORANGE_RED,
            illuminance: 10_000.,
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    commands
        .spawn(FailedCameraBundle {
            camera_bundle: Camera3dBundle {
                transform: Transform::from_xyz(0., 5., -10.).looking_at(Vec3::ZERO, Vec3::Y),
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
                *block = BlockType::Stone
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

    fn construct_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        todo!();
        // let mut verticies = vec!([0., 0., 0., 0., 0.,]);
        mesh
    }
}

enum BlockFace {
    Top,
    Front,
    Right,
    Back,
    Left,
    Bottom,
}

#[derive(Default, PartialEq, Debug)]
enum BlockType {
    #[default]
    Air,
    Grass,
    Stone,
}

struct BlockData {
    is_transparent: bool,
}

impl BlockType {
    fn get_data(&self) -> BlockData {
        BlockData {
            is_transparent: *self == BlockType::Air,
        }
    }
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
) {
    let chunk_coords = IVec3::ZERO;
    let world_seed = 0u64;

    let material = materials.add(StandardMaterial {
        base_color: Color::GRAY,
        perceptual_roughness: 1.,
        reflectance: 0.2,
        ..default()
    });
    let mesh = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));
    let pbr_bundle = PbrBundle {
        // material,
        // mesh: mesh.clone(),
        ..default()
    };

    let chunk_bundle = ChunkBundle {
        pbr: pbr_bundle,
        chunk: Chunk {
            chunk_coords,
            world_seed,
            ..default()
        }
        .generate_terrain(),
    };

    commands.spawn(chunk_bundle);
}

// impl ChunkBundle {
//     fn new(seed: u64, pos: IVec3) -> Self {
//         let c = Chunk {
//             world_seed: seed,
//             chunk_coords: pos,
//             ..default()
//         };

//         let pbr = PbrBundle::default();

//         Self { noise }
//     }
// }

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial::from(Color::GREEN));
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Cube::new(5.).into()),
        material: material.clone(),
        ..Default::default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane { size: 20. }.into()),
        material,
        transform: Transform::from_xyz(0., -5., 0.),
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
