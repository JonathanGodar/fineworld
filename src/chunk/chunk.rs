use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    tasks::Task,
};
use bevy_rapier3d::prelude::*;
use noise::{NoiseFn, OpenSimplex};

use crate::block::{BlockType, UvMappings};

pub const CHUNK_SIZE: IVec3 = IVec3::new(32, 32, 32);
const CHUNK_SIZE_F32: Vec3 = Vec3::new(
    CHUNK_SIZE.x as f32,
    CHUNK_SIZE.y as f32,
    CHUNK_SIZE.z as f32,
);

#[derive(Component)]
pub struct Chunk {
    pub chunk_coords: IVec3,
    pub world_seed: u32,

    // Heap allocated since the stack would overflow otherwise
    pub blocks: Vec<Vec<Vec<BlockType>>>,
}

#[derive(Component)]
pub struct IsInitalMeshGeneration;

#[derive(Component)]
pub struct RequiresMeshGeneration;

#[derive(Component, Deref, DerefMut)]
pub struct GeneratingChunk(pub Task<Chunk>);

impl Chunk {
    pub fn world_coord_chunk(coords: Vec3) -> IVec3 {
        (coords / CHUNK_SIZE_F32).as_ivec3()
    }

    pub fn generate_terrain(mut self) -> Self {
        let noise = OpenSimplex::new(self.world_seed);

        let mut height_map: HashMap<(i32, i32), i32> = HashMap::new();
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

                            ((OCTAVE_HEIGHT as f64 / octave as f64) * (noise.get(sample_point)))
                                as i32
                        })
                        .sum::<i32>()
                        + OCTAVE_HEIGHT,
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

    pub fn set_block(&mut self, pos: IVec3, block_type: BlockType) {
        self.blocks[pos.x as usize][pos.y as usize][pos.z as usize] = block_type;
    }

    // The neighbors should be in the following order: Top, Front, Right, Back, Left, Bottom
    pub fn construct_mesh(
        &self,
        uv_mappings: &UvMappings,
        neighbors: [&Chunk; 6],
    ) -> Option<(Mesh, Collider)> {
        let mut indicies = Vec::new();
        let mut vertecies = Vec::new();

        let mut uvs: Vec<[f32; 2]> = Vec::new();

        for (pos, block) in self.iter_blocks() {
            if self.get_block(pos) == Some(&BlockType::Air) {
                continue;
            }

            let top_visible = self
                .get_block(pos + IVec3::Y)
                .or_else(|| neighbors[0].get_block(IVec3::new(pos.x, 0, pos.z)))
                .map(|block| block.is_transparent())
                .expect("You messed up");

            let front_visible = self
                .get_block(pos + IVec3::Z)
                .or_else(|| neighbors[1].get_block(IVec3::new(pos.x, pos.y, 0)))
                .map(|block| block.is_transparent())
                .expect("You messed up");

            let right_visible = self
                .get_block(pos + IVec3::X)
                .or_else(|| neighbors[2].get_block(IVec3::new(0, pos.y, pos.z)))
                .map(|block| block.is_transparent())
                .expect("You messed up");

            let back_visible = self
                .get_block(pos + IVec3::NEG_Z)
                .or_else(|| neighbors[3].get_block(IVec3::new(pos.x, pos.y, CHUNK_SIZE.z - 1)))
                .map(|block| block.is_transparent())
                .expect("You messed up");

            let left_visible = self
                .get_block(pos + IVec3::NEG_X)
                .or_else(|| neighbors[4].get_block(IVec3::new(CHUNK_SIZE.x - 1, pos.y, pos.z)))
                .map(|block| block.is_transparent())
                .expect("You messed up");

            let bottom_visible = self
                .get_block(pos + IVec3::NEG_Y)
                .or_else(|| neighbors[5].get_block(IVec3::new(pos.x, CHUNK_SIZE.y - 1, pos.z)))
                .map(|block| block.is_transparent())
                .expect("You messed up");

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

#[derive(Component)]
pub struct LoadedChunk;
