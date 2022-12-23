use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_rapier3d::prelude::*;

use crate::{block::BlockType, UvMappings};

const CHUNK_SIZE: IVec3 = IVec3::new(16, 32, 16);
#[derive(Component, Default)]
pub struct Chunk {
    pub chunk_coords: IVec3,
    pub world_seed: u64,
    pub blocks:
        [[[BlockType; CHUNK_SIZE.z as usize]; CHUNK_SIZE.y as usize]; CHUNK_SIZE.x as usize],
}

impl Chunk {
    pub fn generate_terrain(mut self) -> Self {
        for (pos, block) in self.iter_blocks_mut() {
            if pos.y <= 5 && (pos.x < 1 || pos.x > 14 || pos.y == 0 || pos.z > 14 || pos.z < 1) {
                *block = BlockType::Grass
            };
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

    pub fn construct_mesh(&self, uv_mappings: &UvMappings) -> (Mesh, Collider) {
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
            TriMeshFlags::MERGE_DUPLICATE_VERTICES,
        );

        mesh.set_indices(Some(Indices::U32(indicies)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertecies);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        (mesh, collider)
    }
}
