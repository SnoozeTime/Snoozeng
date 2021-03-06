use crate::assets::{Asset, Loader};
use crate::core::transform::Transform;
use hecs::{Entity, World};
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[typetag::serde]
pub trait Prefab: std::fmt::Debug {
    fn spawn(&self, world: &mut hecs::World) -> hecs::Entity;
    fn spawn_with_transform(&self, world: &mut hecs::World, transform: Transform) -> hecs::Entity {
        let e = self.spawn(world);
        world
            .insert_one(e, transform)
            .expect("Cannot add Transform to entity");
        e
    }

    // /// set the position only if the transform is already there
    // fn spawn_at_pos(&self, world: &mut hecs::World, pos: glam::Vec2) -> hecs::Entity {
    //     let e = self.spawn(world);
    //
    //     if let Ok(mut t) = world.get_mut::<Transform>(e) {
    //         t.translation = pos;
    //     }
    //
    //     e
    // }
}

impl Default for Box<dyn Prefab> {
    fn default() -> Self {
        Box::new(EmptyPrefab)
    }
}

// That is necessary for the default implementation. Necessary for std::mem::swap when swapping asset
// from loaded to ready.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyPrefab;
#[typetag::serde]
impl Prefab for EmptyPrefab {
    fn spawn(&self, _world: &mut World) -> Entity {
        unimplemented!()
    }
}

pub struct JsonSyncLoader {
    base_path: PathBuf,
}

impl JsonSyncLoader {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        let base_path = base_path.as_ref();
        Self {
            base_path: base_path.to_path_buf(),
        }
    }
}

impl<T> Loader<T> for JsonSyncLoader
where
    T: Default + DeserializeOwned,
{
    fn load(&mut self, asset_name: String) -> Asset<T> {
        let mut asset = Asset::new();
        let asset_path = self.base_path.join(&asset_name).with_extension("json");
        info!("Will load at path = {}", asset_path.display());

        match std::fs::read_to_string(asset_path) {
            Ok(asset_str) => {
                let res: Result<T, _> = serde_json::from_str(&asset_str);
                match res {
                    Ok(val) => {
                        info!("Finished loading {}", asset_name);
                        asset.set_loaded(val)
                    }
                    Err(e) => {
                        error!("Error while converting prefab from json = {:?}", e);
                        asset.set_error(e.into())
                    }
                }
            }
            Err(e) => {
                error!("Error while reading from file = {:?}", e);
                asset.set_error(e.into())
            }
        }

        asset
    }
}
