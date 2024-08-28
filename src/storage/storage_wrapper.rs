use std::collections::HashMap;

use anyhow::Result;

use crate::types::position::{self, Position};

use super::json::JsonStorage;

pub trait Storage {
    fn load_state(&mut self) -> Result<(u64, HashMap<u64, Position>)>;
    fn save_state(
        &mut self,
        position: HashMap<u64, position::Position>,
        last_block_indexed: u64,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get_last_saved_positions_map(&self) -> HashMap<u64, Position>;
}

pub enum StorageWrapper {
    Json(JsonStorage),
}

impl Storage for StorageWrapper {
    fn load_state(&mut self) -> Result<(u64, HashMap<u64, Position>)> {
        match self {
            StorageWrapper::Json(json_storage) => json_storage.load_state(),
        }
    }

    async fn save_state(
        &mut self,
        position: HashMap<u64, position::Position>,
        last_block_indexed: u64,
    ) -> Result<()> {
        match self {
            StorageWrapper::Json(json_storage) => {
                json_storage.save_state(position, last_block_indexed).await
            }
        }
    }

    fn get_last_saved_positions_map(&self) -> HashMap<u64, Position> {
        match self {
            StorageWrapper::Json(json_storage) => json_storage.get_last_saved_positions_map(),
        }
    }
}
