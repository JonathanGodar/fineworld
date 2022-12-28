pub mod textures;

use std::{collections::HashMap, sync::Arc};

use bevy::prelude::*;

pub type UVs = [[f32; 2]; 4];
#[derive(Resource, Deref, DerefMut, Debug)]
pub struct UvMappingsRes(pub Arc<UvMappings>);

pub type UvMappings = HashMap<BlockType, (UVs, UVs, UVs)>;

#[derive(Default, PartialEq, Debug, Reflect, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum BlockType {
    #[default]
    Air,
    Grass,
    Stone,
    Placeholder,
}

impl BlockType {
    pub fn is_transparent(&self) -> bool {
        return *self == BlockType::Air;
    }
}

#[derive(Deref, DerefMut, Resource, Default)]
pub struct BlockTextureHandles(Vec<HandleUntyped>);

#[derive(Resource, DerefMut, Deref, Clone)]
pub struct BlockAtlasHandle(Handle<TextureAtlas>);
