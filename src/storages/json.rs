use std::{fs::File, io::Write, path::PathBuf};

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

use crate::types::position::{self, Position};

use super::{Storage, StoredData};

pub struct JsonStorage {
    file_path: PathBuf,
    data: StoredData,
}

impl JsonStorage {
    pub fn new(path: &str) -> Self {
        JsonStorage {
            file_path: PathBuf::from(path),
            data: StoredData::default(),
        }
    }
}

#[async_trait::async_trait]
impl Storage for JsonStorage {
    async fn load(&mut self) -> Result<(u64, HashMap<u64, Position>)> {
        if !self.file_path.exists() {
            self.data = StoredData::new(0, HashMap::new());
            return Ok(self.data.as_tuple());
        }
        let json_value: Value = serde_json::from_reader(File::open(self.file_path.clone()).expect("failed to open file")).expect("failed to load json from file reader");
        let last_block_indexed: u64 = match json_value.get("last_block_indexed") {
            Some(Value::Number(lbi)) => {
                if lbi.is_u64() {
                    lbi.as_u64().unwrap()
                } else {
                    0_u64
                }
            }
            _ => 0_u64,
        };
        // no need to go further if last block indexed is genesis
        if last_block_indexed == 0 {
            self.data = StoredData::new(0, HashMap::new());
            return Ok(self.data.as_tuple());
        }
        let positions: HashMap<u64, Position> = match json_value.get("positions") {
            Some(Value::Object(map)) => map
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.parse::<u64>().ok()?;
                    let position: Position = serde_json::from_value(value.clone()).ok()?;
                    Some((key, position))
                })
                .collect(),
            _ => HashMap::new(),
        };
        self.data = StoredData::new(last_block_indexed, positions);
        Ok(self.data.as_tuple())
    }

    async fn save(
        &mut self,
        positions: HashMap<u64, position::Position>,
        last_block_indexed: u64,
    ) -> Result<()> {
        let file_path = self.file_path.clone();
        let map = StoredData {
            last_block_indexed,
            positions,
        };
        let json = serde_json::to_string_pretty(&map)?;
        let mut file = File::create(file_path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    fn get_positions(&self) -> HashMap<u64, Position> {
        self.data.positions.clone()
    }
}
