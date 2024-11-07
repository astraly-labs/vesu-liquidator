pub mod json;

use std::collections::HashMap;

use anyhow::Result;
use dashmap::DashMap;

use crate::types::position::{self, Position};

#[derive(serde::Serialize, Default)]
struct StoredData {
    last_block_indexed: u64,
    positions: HashMap<u64, Position>,
}

impl StoredData {
    pub fn new(last_block_indexed: u64, positions: HashMap<u64, Position>) -> Self {
        StoredData {
            last_block_indexed,
            positions,
        }
    }
    pub fn as_tuple(&self) -> (u64, HashMap<u64, Position>) {
        (self.last_block_indexed, self.positions.clone())
    }
}

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn load(&mut self) -> Result<(u64, HashMap<u64, Position>)>;
    async fn save(
        &mut self,
        positions: &DashMap<u64, position::Position>,
        last_block_indexed: u64,
    ) -> Result<()>;
    fn get_positions(&self) -> HashMap<u64, Position>;
}
