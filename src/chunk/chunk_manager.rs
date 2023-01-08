use std::collections::{HashMap, HashSet};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use bevy_rapier3d::prelude::*;
use futures_lite::future;

use crate::{
    block::{textures::ChunkMaterialHandle, BlockType, UvMappingsRes},
    player::Player,
};

use super::chunk::{
    Chunk, GeneratingChunk, IsInitalMeshGeneration, LoadedChunk, RequiresMeshGeneration, CHUNK_SIZE,
};

#[derive(Resource, Default)]
pub struct ChunkManager {
    queued_block_breaks: HashMap<IVec3, Vec<IVec3>>,
}

const RENDER_DISTANCE: i32 = 3;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct LoadedChunks(HashMap<IVec3, Entity>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct GeneratingChunks(HashMap<IVec3, Entity>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct WorldSeed(u32);

impl ChunkManager {
    pub fn break_block(&mut self, world_pos: IVec3) {
        let chunk = world_pos / CHUNK_SIZE;
        let block = world_pos % CHUNK_SIZE;

        self.queued_block_breaks
            .entry(chunk)
            .or_default()
            .push(block);
    }
}

pub fn handle_chunk_updates(
    mut commands: Commands,
    mut chunks: Query<&mut Chunk>,
    mut manager: ResMut<ChunkManager>,
    loaded_chunks: Res<LoadedChunks>,
) {
    // chunks.get_many();
    let mut chunks_need_updating = HashSet::new();
    // let chunks = query.
    for (chunk_coord, blocks) in manager.queued_block_breaks.drain() {
        let chunk_id = {
            let opt = loaded_chunks.get(&chunk_coord);
            if opt.is_none() {
                warn!("Tried to break block in an unloaded chunk");
                continue;
            }
            opt.unwrap()
        };
        let mut chunk = chunks.get_mut(*chunk_id).unwrap();

        for block in blocks {
            chunk.set_block(block, BlockType::Air);
            // chunk.blocks[{}]
            // chunks
        }

        chunks_need_updating.insert(*chunk_id);
    }

    for chnk in chunks_need_updating.drain() {
        commands
            .get_entity(chnk)
            .unwrap()
            .insert(RequiresMeshGeneration {});
    }
}

// pub fn update_chu

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
        for y in -LOAD_DISTANCE / 2..LOAD_DISTANCE / 2 {
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
                        blocks: vec![
                            vec![
                                vec![BlockType::Air; CHUNK_SIZE.z as usize];
                                CHUNK_SIZE.y as usize
                            ];
                            CHUNK_SIZE.x as usize
                        ],
                    };

                    chunk.generate_terrain()
                });

                let entity = commands.spawn(GeneratingChunk(chunk_generation_task)).id();
                loading_chunks.0.insert(curr_chunk_coord, entity);
            }
        }
    }
}

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

pub fn handle_generated_chunks_system(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GeneratingChunk)>,
    mut generating_chunks: ResMut<GeneratingChunks>,
    mut loaded_chunks: ResMut<LoadedChunks>,
) {
    for (entity, mut generating_chunk) in q.iter_mut() {
        if let Some(chunk) = future::block_on(future::poll_once(&mut **generating_chunk)) {
            loaded_chunks.insert(
                chunk.chunk_coords,
                generating_chunks
                    .remove_entry(&chunk.chunk_coords)
                    .unwrap()
                    .1,
            );
            commands.entity(entity).remove::<GeneratingChunk>();
            commands
                .entity(entity)
                .insert(TransformBundle {
                    local: Transform::from_translation(chunk.get_world_coords()),
                    ..default()
                })
                .insert((
                    chunk,
                    RequiresMeshGeneration,
                    IsInitalMeshGeneration,
                    CollisionGroups::new(Group::GROUP_2, Group::ALL),
                ));
        }
    }
}

pub fn mesh_generation_system(
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
        if !neighbors.iter().all(|f| f.is_some()) {
            return;
        };
        let neighbors = q_all_chunks.get_many(neighbors.map(|n| *n.unwrap()));
        if neighbors.is_err() {
            return;
        };
        let neighbors = neighbors.unwrap();

        let constructed = chnk.construct_mesh(&uv_mappings, neighbors);
        let mut e = commands.entity(e_id);
        e.remove::<RequiresMeshGeneration>();
        if let Some((mesh, collider)) = constructed {
            let material = (**material).clone();

            let pbr_bundle = PbrBundle {
                material,
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
        }
    });
}
