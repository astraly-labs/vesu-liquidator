use std::{collections::HashMap, pin::Pin};

use anyhow::Result;
use futures_util::Future;

use crate::types::position::{self, Position};

pub trait Storage: Send + Sync {
    fn load_state(&mut self) -> Result<(u64, HashMap<u64, Position>)>;
    fn save_state(
        &mut self,
        position: HashMap<u64, position::Position>,
        last_block_indexed: u64,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn get_last_saved_positions_map(&self) -> HashMap<u64, Position>;
}
