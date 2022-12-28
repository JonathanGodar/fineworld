use bevy::prelude::*;

#[derive(Resource, Deref, DerefMut, Default)]
pub struct WorldSeed(u32);
