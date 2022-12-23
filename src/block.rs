use bevy::{prelude::*, utils::HashMap};

pub type UVs = [[f32; 2]; 4];
#[derive(Resource, Deref, DerefMut, Default, Debug)]
pub struct UvMappings(pub HashMap<BlockType, (UVs, UVs, UVs)>);

#[derive(Default, PartialEq, Debug, Reflect, Eq, PartialOrd, Ord, Hash)]
pub enum BlockType {
    #[default]
    Air,
    Grass,
    Stone,
    Placeholder,
}
