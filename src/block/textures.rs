
use std::{collections::HashMap, sync::Arc};

use bevy::{
    asset::LoadState,
    prelude::*,
    reflect::{DynamicEnum, DynamicVariant},
};

use crate::{
    block::{BlockAtlasHandle, BlockTextureHandles, BlockType, UVs, UvMappingsRes},
    AppState,
};

pub mod textures {
    use super::*;

    pub fn construct_atlas(
        mut commands: Commands,
        block_texture_handles: Res<BlockTextureHandles>,
        asset_server: Res<AssetServer>,
        mut texture_atlases: ResMut<Assets<TextureAtlas>>,
        mut textures: ResMut<Assets<Image>>,
        mut standard_material: ResMut<Assets<StandardMaterial>>,
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
                    let top_right = ((image_rect.min + Vec2::new(image_rect.width(), 0.))
                        / atlas_size)
                        .to_array();

                    let bottom_right = (image_rect.max / atlas_size).to_array();
                    let bottom_left = ((image_rect.min + Vec2::new(0., image_rect.height()))
                        / atlas_size)
                        .to_array();

                    [top_left, top_right, bottom_right, bottom_left]
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

        let mut uv_mappings = HashMap::default();
        {
            let placeholder = textures.remove("Placeholder");
            uv_mappings.insert(BlockType::Placeholder, placeholder.unwrap());
        }

        for entry in textures {
            let mut curr_type = BlockType::Air;
            let dynamic_enum = DynamicEnum::new(
                Reflect::type_name(&BlockType::Air),
                &entry.0,
                DynamicVariant::Unit,
            );

            curr_type.apply(&dynamic_enum);

            uv_mappings.insert(curr_type, entry.1);
        }

        commands.insert_resource(ChunkMaterialHandle(standard_material.add(StandardMaterial {
            base_color: Color::ORANGE,
            base_color_texture: Some(texture_atlas.texture.clone()),
            unlit: true,
            ..default()
        })));
        commands.insert_resource(BlockAtlasHandle(texture_atlases.add(texture_atlas)));
        commands.insert_resource(UvMappingsRes(Arc::new(uv_mappings)));
        commands.remove_resource::<BlockTextureHandles>();
    }

    pub fn validate_textures(
        mut state: ResMut<State<AppState>>,
        asset_server: Res<AssetServer>,
        block_texture_handles: Res<BlockTextureHandles>,
        commands: Commands,
        texture_atlases: ResMut<Assets<TextureAtlas>>,
        textures: ResMut<Assets<Image>>,
        materials: ResMut<Assets<StandardMaterial>>,
    ) {
        if let LoadState::Loaded =
            asset_server.get_group_load_state(block_texture_handles.iter().map(|h| h.id))
        {
            state.set(AppState::Game).unwrap();
            construct_atlas(
                commands,
                block_texture_handles,
                asset_server,
                texture_atlases,
                textures,
                materials,
            );
        }
    }

    #[derive(Deref, DerefMut)]
    struct BlocksAtlas(TextureAtlas);

    pub fn load_textures(
        asset_server: Res<AssetServer>,
        mut block_texture_handles: ResMut<BlockTextureHandles>,
    ) {
        block_texture_handles.0 = asset_server.load_folder("./textures/blocks").unwrap();
    }
}


#[derive(Resource, Deref, DerefMut)]
pub struct ChunkMaterialHandle(Handle<StandardMaterial>);
