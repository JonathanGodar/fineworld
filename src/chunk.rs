use std::{collections::HashMap, time::Duration};

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task}, pbr::wireframe::Wireframe,
};
use bevy_rapier3d::prelude::*;
use futures_lite::future;
use noise::{NoiseFn, OpenSimplex};

use crate::{
    block::{BlockType, UvMappings, BlockAtlasHandle, textures::ChunkMaterialHandle},
    game_world::WorldSeed,
    player::Player,
    UvMappingsRes,
};

const CHUNK_SIZE: IVec3 = IVec3::new(32, 32, 32);
const CHUNK_SIZE_F32: Vec3 = Vec3::new(
    CHUNK_SIZE.x as f32,
    CHUNK_SIZE.y as f32,
    CHUNK_SIZE.z as f32,
);

const RENDER_DISTANCE: i32 = 3;


#[derive(Component)]
pub struct Chunk {
    pub chunk_coords: IVec3,
    pub world_seed: u32,

    // Heap allocated since the stack would overflow otherwise
    pub blocks: Vec<Vec<Vec<BlockType>>>,
        // [[[BlockType; CHUNK_SIZE.z as usize]; CHUNK_SIZE.y as usize]; CHUNK_SIZE.x as usize],
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct LoadedChunks(HashMap<IVec3, Entity>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct GeneratingChunks(HashMap<IVec3, Entity>);

#[derive(Component)]
pub struct IsInitalMeshGeneration;

#[derive(Component)]
pub struct RequiresMeshGeneration;

#[derive(Component, Deref, DerefMut)]
pub struct GeneratingChunk(Task<Chunk>);

// Side chunks with the same ordering as blocks; Top, Front, Right, Back, Left, Bottom
struct SideChunks<'a>(&'a Chunk, &'a Chunk, &'a Chunk, &'a Chunk, &'a Chunk, &'a Chunk);


/*
Generate the terrain info for all blocks, from the inside and going outwards,
If the chunk has four loaded neigbors, construct its mesh.
*/

impl Chunk {
    pub fn world_coord_chunk(coords: Vec3) -> IVec3 {
        (coords / CHUNK_SIZE_F32).as_ivec3()
    }

    pub fn generate_terrain(mut self) -> Self {
        let noise = OpenSimplex::new(self.world_seed);

        let mut height_map: HashMap<(i32, i32), i32>= HashMap::new();
        for x in 0..(CHUNK_SIZE.x) {
            for z in 0..(CHUNK_SIZE.z) {
                let world_x = (x + self.chunk_coords.x * CHUNK_SIZE.x) as f64;
                let world_z = (z + self.chunk_coords.z * CHUNK_SIZE.z) as f64;

                const OCTAVES: i32 = 1;
                const OCTAVE_HEIGHT: i32 = 20;
                height_map.insert(
                    (x, z),
                    (1..=OCTAVES)
                        .map(|octave| {
                            let sample_point = [
                                (world_x / (500. / octave as f64)),
                                (world_z / (500. / octave as f64)),
                            ];

                            ((OCTAVE_HEIGHT as f64 / octave as f64) * (noise.get(sample_point))) as i32
                        })
                        .sum::<i32>() + OCTAVE_HEIGHT,
                );
            }
        }

        let chunk_world_y = self.chunk_coords.y * CHUNK_SIZE.y;
        for (pos, block) in self.iter_blocks_mut() {
            if pos.y + chunk_world_y <= height_map[&(pos.x, pos.z)] {
                *block = BlockType::Grass;
            }
        }
        self
    }

    pub fn iter_blocks_mut(&mut self) -> impl Iterator<Item = (IVec3, &mut BlockType)> {
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

    pub fn iter_blocks(&self) -> impl Iterator<Item = (IVec3, &BlockType)> {
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

    pub fn get_block(&self, pos: IVec3) -> Option<&BlockType> {
        if !Chunk::is_within_bounds(pos) {
            return None;
        }

        return Some(&self.blocks[pos.x as usize][pos.y as usize][pos.z as usize]);
    }

    pub fn get_block_mut(&mut self, pos: IVec3) -> Option<&mut BlockType> {
        if !Chunk::is_within_bounds(pos) {
            return None;
        }

        return Some(&mut self.blocks[pos.x as usize][pos.y as usize][pos.z as usize]);
    }

    #[inline]
    pub fn is_within_bounds(pos: IVec3) -> bool {
        return pos.x >= 0
            && pos.x < CHUNK_SIZE.x
            && pos.y >= 0
            && pos.y < CHUNK_SIZE.y
            && pos.z >= 0
            && pos.z < CHUNK_SIZE.z;
    }

    // The neighbors should be in the following order: Top, Front, Right, Back, Left, Bottom
    pub fn construct_mesh(&self, uv_mappings: &UvMappings, neighbors: [&Chunk; 6]) -> Option<(Mesh, Collider)> {
        let mut indicies = Vec::new();
        let mut vertecies = Vec::new();

        let mut uvs: Vec<[f32; 2]> = Vec::new();

        for (pos, block) in self.iter_blocks() {
            if self.get_block(pos) == Some(&BlockType::Air) {
                continue;
            }

            let top_visible = self
                .get_block(pos + IVec3::Y).or_else(
                    || neighbors[0].get_block(IVec3::new(pos.x, 0, pos.z))
                ).map(|block| block.is_transparent()).expect("You messed up");

            let front_visible = self
                .get_block(pos + IVec3::Z).or_else(
                    || neighbors[1].get_block(IVec3::new(pos.x, pos.y, 0))
                ).map(|block| block.is_transparent()).expect("You messed up");


            let right_visible = self
                .get_block(pos + IVec3::X).or_else(
                    || neighbors[2].get_block(IVec3::new(0, pos.y, pos.z))
                ).map(|block| block.is_transparent()).expect("You messed up");

            let back_visible = self
                .get_block(pos + IVec3::NEG_Z).or_else(
                    || neighbors[3].get_block(IVec3::new(pos.x, pos.y, CHUNK_SIZE.z - 1))
                ).map(|block| block.is_transparent()).expect("You messed up");

            let left_visible = self
                .get_block(pos + IVec3::NEG_X).or_else(
                    || neighbors[4].get_block(IVec3::new(CHUNK_SIZE.x - 1, pos.y, pos.z))
                ).map(|block| block.is_transparent()).expect("You messed up");

            let bottom_visible = self
                .get_block(pos + IVec3::NEG_Y).or_else(
                    || neighbors[5].get_block(IVec3::new(pos.x, CHUNK_SIZE.y - 1, pos.z))
                ).map(|block| block.is_transparent()).expect("You messed up");

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
        }

        if indicies.len() == 0 {
            return None;
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        // TODO Implement Qua
        let collider = Collider::trimesh_with_flags(
            vertecies.clone().into_iter().map(|m| m.into()).collect(),
            indicies
                .chunks_exact(3)
                .map(|chunk| {
                    let mut arr = [0; 3];
                    arr.copy_from_slice(chunk);
                    arr
                })
                .collect(),
            TriMeshFlags::all(),
        );

        mesh.set_indices(Some(Indices::U32(indicies)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertecies);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        Some((mesh, collider))
    }

    pub fn get_world_coords(&self) -> Vec3 {
        self.chunk_coords.as_vec3() * CHUNK_SIZE_F32
    }

}

pub fn chunk_load_system(
    mut commands: Commands,
    world_seed: Res<WorldSeed>,
    mappings_res: Res<UvMappingsRes>,
    player: Query<&Transform, With<Player>>,
    mut loading_chunks: ResMut<GeneratingChunks>,
    loaded_chunks: Res<LoadedChunks>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let player_chunk_coord = Chunk::world_coord_chunk(player.get_single().unwrap().translation);

    const LOAD_DISTANCE: i32 = RENDER_DISTANCE + 1;
    for x in -LOAD_DISTANCE..LOAD_DISTANCE {
        for y in -LOAD_DISTANCE/2..LOAD_DISTANCE/2 {
            for z in -LOAD_DISTANCE..LOAD_DISTANCE {
            let curr_chunk_coord = player_chunk_coord + IVec3::new(x, y, z);
            if loaded_chunks.0.contains_key(&curr_chunk_coord)
                || loading_chunks.0.contains_key(&curr_chunk_coord)
            {
                continue;
            }

            println!("Loading chunk {curr_chunk_coord:?}"); 
            let world_seed = **world_seed;
            let mappings = (mappings_res).clone();
            let chunk_generation_task = task_pool.spawn(async move {
                let mut chunk = Chunk {
                    world_seed: world_seed,
                    chunk_coords: curr_chunk_coord,
                    blocks: vec![vec![vec![BlockType::Air; CHUNK_SIZE.z as usize]; CHUNK_SIZE.y as usize]; CHUNK_SIZE.x as usize],
                };

                chunk.generate_terrain()
            });

            let entity = commands.spawn(GeneratingChunk(chunk_generation_task)).id();
            loading_chunks.0.insert(curr_chunk_coord, entity);
        }
        }
    }
}

pub fn handle_generated_chunks_system(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GeneratingChunk)>,
    mut generating_chunks: ResMut<GeneratingChunks>,
    mut loaded_chunks: ResMut<LoadedChunks>,
) {
    for (entity, mut generating_chunk) in q.iter_mut() {
        if let Some(chunk) = future::block_on(future::poll_once(&mut **generating_chunk)){
            println!("handling {:?}", chunk.chunk_coords);
            loaded_chunks.insert(
                chunk.chunk_coords,
                generating_chunks.remove_entry(&chunk.chunk_coords).unwrap().1
            );
            commands.entity(entity).remove::<GeneratingChunk>();
            commands.entity(entity).insert(TransformBundle{
                local: Transform::from_translation(chunk.get_world_coords()),
                ..default()
            }).insert(
                (chunk, RequiresMeshGeneration, IsInitalMeshGeneration)
            );
        }
    }
}

pub fn queue_mesh_generation_system(
    mut commands: Commands,
    
    q: Query<(Entity, &Chunk, Option<&IsInitalMeshGeneration>), With<RequiresMeshGeneration>>,
    q_all_chunks: Query<&Chunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterialHandle>,
    uv_mappings: Res<UvMappingsRes>,
    loaded_chunks: Res<LoadedChunks>,
) {
    q.for_each(|(e_id, chnk, is_inital)| {
        let dirs = [
            IVec3::new(0, 1, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(1, 0, 0),
            IVec3::new(0, 0, -1),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, -1, 0),
        ];

        let neighbors = dirs.map(|dir| loaded_chunks.get(&(dir + chnk.chunk_coords)));
        if !neighbors.iter().all(|f| f.is_some()) { return; };
        let neighbors = q_all_chunks.get_many(neighbors.map(|n| *n.unwrap()));
        if neighbors.is_err() { return; };
        let neighbors = neighbors.unwrap();

        let constructed = chnk.construct_mesh(&uv_mappings, neighbors);
        let mut e = commands.entity(e_id);
        e.remove::<RequiresMeshGeneration>();
        // print!("trying to construct mesh");
        if let Some((mesh, collider)) = constructed {
            // let neighbors = q.get_
            println!("Generating mesh {:?}", chnk.chunk_coords);
            let material = (**material).clone();

            // e.insert(material).insert(meshes.add(mesh));
            
            let pbr_bundle = PbrBundle {
                material: material,
                transform: Transform::from_translation(chnk.get_world_coords()),
                mesh: meshes.add(mesh),
                ..default()
            };

            e.insert(pbr_bundle);
            e.insert(collider);
            e.insert(RigidBody::Fixed);
        }

        if is_inital.is_some() {
            e.remove::<IsInitalMeshGeneration>();
            e.insert(LoadedChunk);
            // evw.send(ChunkLoadedEvent(e.id()));
        } 
    });
}


#[derive(Component)]
pub struct LoadedChunk;























// ------------------- Old(but multithreaded) chunk loading system
// pub fn process_chunk_tasks_system(
//     mut q: Query<(Entity, &mut GeneratingChunk)>,
//     mut commands: Commands,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut loading_chunks: ResMut<GeneratingChunks>,
//     mut loaded_chunks: ResMut<LoadedChunks>,
//     atlas_handle: Res<BlockAtlasHandle>,
//     texture_atlases: Res<Assets<TextureAtlas>>,
// ) {
//     for (e, mut task) in q.iter_mut() {
//         let atlas_image_handle = texture_atlases.get(&atlas_handle).unwrap().texture.clone();


//         let material = materials.add(StandardMaterial {
//             base_color: Color::ORANGE,
//             base_color_texture: Some(atlas_image_handle),
//             unlit: true,
//             ..default()
//         });

//         if let Some(task_result) = future::block_on(future::poll_once(&mut task.0)) {
//             let mut entity = commands.entity(e);
//             entity.remove::<GeneratingChunk>();
//             if task_result.0.is_none() {
//                 entity.insert(task_result.1);
//                 return;
//             }

//             let (mesh, collider) = task_result.0.unwrap();
//             let chunk = task_result.1;

//             let pbr_bundle = PbrBundle {
//                 material,
//                 mesh: meshes.add(mesh),
//                 transform: Transform::from_translation(
//                     chunk.chunk_coords.as_vec3() * CHUNK_SIZE_F32,
//                 ),
//                 ..default()
//             };

//             loaded_chunks.0.insert(
//                 chunk.chunk_coords,
//                 loading_chunks.0.remove(&chunk.chunk_coords).unwrap(),
//             );

//             let chunk_bundle = ( 
//                 pbr_bundle,
//                 chunk,
//             );

//             entity
//                 .insert(chunk_bundle)
//                 .insert(collider)
//                 .insert(RigidBody::Fixed);
//         }
//     }
// }

// #[derive(Component, Deref, DerefMut)]
// pub struct GeneratingChunk(Task<(Option<(Mesh, Collider)>, Chunk)>);


// pub fn chunk_load_system(
//     mut commands: Commands,
//     world_seed: Res<WorldSeed>,
//     mappings_res: Res<UvMappingsRes>,
//     player: Query<&Transform, With<Player>>,
//     mut loading_chunks: ResMut<GeneratingChunks>,
//     loaded_chunks: Res<LoadedChunks>,
// ) {
//     let task_pool = AsyncComputeTaskPool::get();
//     let player_chunk_coord = Chunk::world_coord_chunk(player.get_single().unwrap().translation);

//     const LOAD_DISTANCE: i32 = RENDER_DISTANCE + 1;
//     for x in -LOAD_DISTANCE..LOAD_DISTANCE {
//         for y in -LOAD_DISTANCE/2..LOAD_DISTANCE/2 {
//             for z in -LOAD_DISTANCE..LOAD_DISTANCE {
//             let curr_chunk_coord = player_chunk_coord + IVec3::new(x, y, z);
//             if loaded_chunks.0.contains_key(&curr_chunk_coord)
//                 || loading_chunks.0.contains_key(&curr_chunk_coord)
//             {
//                 continue;
//             }

//             println!("Loading chunk {curr_chunk_coord:?}"); 
//             let world_seed = **world_seed;
//             let mappings = (mappings_res).clone();
//             let chunk_generation_task = task_pool.spawn(async move {
//                 let mut chunk = Chunk {
//                     world_seed: world_seed,
//                     chunk_coords: curr_chunk_coord,
//                     blocks: vec![vec![vec![BlockType::Air; CHUNK_SIZE.z as usize]; CHUNK_SIZE.y as usize]; CHUNK_SIZE.x as usize],
//                 };

//                 chunk = chunk.generate_terrain();
//                 (chunk.construct_mesh(&mappings), chunk)
//             });

//             let entity = commands.spawn(GeneratingChunk(chunk_generation_task)).id();
//             loading_chunks.0.insert(curr_chunk_coord, entity);
//         }
//         }
//     }
// }


pub fn chunk_unload_system(
    mut commands: Commands,
    mut loaded_chunks: ResMut<LoadedChunks>,
    player_query: Query<&Transform, With<Player>>,
) {
    const RENDER_DISTANCE: i32 = 10;
    let player_chunk_coord = Chunk::world_coord_chunk(player_query.single().translation);

    loaded_chunks.0.retain(|coord, entity| {
        let diff = (*coord - player_chunk_coord).abs();
        if diff.x > RENDER_DISTANCE || diff.y > RENDER_DISTANCE || diff.z > RENDER_DISTANCE {
            commands.entity(*entity).despawn();
            return false;
        }
        return true;
    });
}
