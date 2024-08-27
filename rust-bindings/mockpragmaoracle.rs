#[derive(Debug)]
pub struct MockPragmaOracle<A: starknet::accounts::ConnectedAccount + Sync> {
    pub address: starknet::core::types::Felt,
    pub account: A,
    pub block_id: starknet::core::types::BlockId,
}
impl<A: starknet::accounts::ConnectedAccount + Sync> MockPragmaOracle<A> {
    pub fn new(address: starknet::core::types::Felt, account: A) -> Self {
        Self {
            address,
            account,
            block_id: starknet::core::types::BlockId::Tag(
                starknet::core::types::BlockTag::Pending,
            ),
        }
    }
    pub fn set_contract_address(&mut self, address: starknet::core::types::Felt) {
        self.address = address;
    }
    pub fn provider(&self) -> &A::Provider {
        self.account.provider()
    }
    pub fn set_block(&mut self, block_id: starknet::core::types::BlockId) {
        self.block_id = block_id;
    }
    pub fn with_block(self, block_id: starknet::core::types::BlockId) -> Self {
        Self { block_id, ..self }
    }
}
#[derive(Debug)]
pub struct MockPragmaOracleReader<P: starknet::providers::Provider + Sync> {
    pub address: starknet::core::types::Felt,
    pub provider: P,
    pub block_id: starknet::core::types::BlockId,
}
impl<P: starknet::providers::Provider + Sync> MockPragmaOracleReader<P> {
    pub fn new(address: starknet::core::types::Felt, provider: P) -> Self {
        Self {
            address,
            provider,
            block_id: starknet::core::types::BlockId::Tag(
                starknet::core::types::BlockTag::Pending,
            ),
        }
    }
    pub fn set_contract_address(&mut self, address: starknet::core::types::Felt) {
        self.address = address;
    }
    pub fn provider(&self) -> &P {
        &self.provider
    }
    pub fn set_block(&mut self, block_id: starknet::core::types::BlockId) {
        self.block_id = block_id;
    }
    pub fn with_block(self, block_id: starknet::core::types::BlockId) -> Self {
        Self { block_id, ..self }
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct PragmaPricesResponse {
    pub price: u128,
    pub decimals: u32,
    pub last_updated_timestamp: u64,
    pub num_sources_aggregated: u32,
    pub expiration_timestamp: Option<u64>,
}
impl cainome::cairo_serde::CairoSerde for PragmaPricesResponse {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += u128::cairo_serialized_size(&__rust.price);
        __size += u32::cairo_serialized_size(&__rust.decimals);
        __size += u64::cairo_serialized_size(&__rust.last_updated_timestamp);
        __size += u32::cairo_serialized_size(&__rust.num_sources_aggregated);
        __size += Option::<u64>::cairo_serialized_size(&__rust.expiration_timestamp);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(u128::cairo_serialize(&__rust.price));
        __out.extend(u32::cairo_serialize(&__rust.decimals));
        __out.extend(u64::cairo_serialize(&__rust.last_updated_timestamp));
        __out.extend(u32::cairo_serialize(&__rust.num_sources_aggregated));
        __out.extend(Option::<u64>::cairo_serialize(&__rust.expiration_timestamp));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let price = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&price);
        let decimals = u32::cairo_deserialize(__felts, __offset)?;
        __offset += u32::cairo_serialized_size(&decimals);
        let last_updated_timestamp = u64::cairo_deserialize(__felts, __offset)?;
        __offset += u64::cairo_serialized_size(&last_updated_timestamp);
        let num_sources_aggregated = u32::cairo_deserialize(__felts, __offset)?;
        __offset += u32::cairo_serialized_size(&num_sources_aggregated);
        let expiration_timestamp = Option::<u64>::cairo_deserialize(__felts, __offset)?;
        __offset += Option::<u64>::cairo_serialized_size(&expiration_timestamp);
        Ok(PragmaPricesResponse {
            price,
            decimals,
            last_updated_timestamp,
            num_sources_aggregated,
            expiration_timestamp,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct BaseEntry {
    pub timestamp: u64,
    pub source: starknet::core::types::Felt,
    pub publisher: starknet::core::types::Felt,
}
impl cainome::cairo_serde::CairoSerde for BaseEntry {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += u64::cairo_serialized_size(&__rust.timestamp);
        __size += starknet::core::types::Felt::cairo_serialized_size(&__rust.source);
        __size += starknet::core::types::Felt::cairo_serialized_size(&__rust.publisher);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(u64::cairo_serialize(&__rust.timestamp));
        __out.extend(starknet::core::types::Felt::cairo_serialize(&__rust.source));
        __out.extend(starknet::core::types::Felt::cairo_serialize(&__rust.publisher));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let timestamp = u64::cairo_deserialize(__felts, __offset)?;
        __offset += u64::cairo_serialized_size(&timestamp);
        let source = starknet::core::types::Felt::cairo_deserialize(__felts, __offset)?;
        __offset += starknet::core::types::Felt::cairo_serialized_size(&source);
        let publisher = starknet::core::types::Felt::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += starknet::core::types::Felt::cairo_serialized_size(&publisher);
        Ok(BaseEntry {
            timestamp,
            source,
            publisher,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpotEntry {
    pub base: BaseEntry,
    pub price: u128,
    pub pair_id: starknet::core::types::Felt,
    pub volume: u128,
}
impl cainome::cairo_serde::CairoSerde for SpotEntry {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += BaseEntry::cairo_serialized_size(&__rust.base);
        __size += u128::cairo_serialized_size(&__rust.price);
        __size += starknet::core::types::Felt::cairo_serialized_size(&__rust.pair_id);
        __size += u128::cairo_serialized_size(&__rust.volume);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(BaseEntry::cairo_serialize(&__rust.base));
        __out.extend(u128::cairo_serialize(&__rust.price));
        __out.extend(starknet::core::types::Felt::cairo_serialize(&__rust.pair_id));
        __out.extend(u128::cairo_serialize(&__rust.volume));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let base = BaseEntry::cairo_deserialize(__felts, __offset)?;
        __offset += BaseEntry::cairo_serialized_size(&base);
        let price = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&price);
        let pair_id = starknet::core::types::Felt::cairo_deserialize(__felts, __offset)?;
        __offset += starknet::core::types::Felt::cairo_serialized_size(&pair_id);
        let volume = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&volume);
        Ok(SpotEntry {
            base,
            price,
            pair_id,
            volume,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubmittedSpotEntry {
    pub spot_entry: SpotEntry,
}
impl cainome::cairo_serde::CairoSerde for SubmittedSpotEntry {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += SpotEntry::cairo_serialized_size(&__rust.spot_entry);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(SpotEntry::cairo_serialize(&__rust.spot_entry));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let spot_entry = SpotEntry::cairo_deserialize(__felts, __offset)?;
        __offset += SpotEntry::cairo_serialized_size(&spot_entry);
        Ok(SubmittedSpotEntry { spot_entry })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub enum Event {
    SubmittedSpotEntry(SubmittedSpotEntry),
}
impl cainome::cairo_serde::CairoSerde for Event {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = std::option::Option::None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        match __rust {
            Event::SubmittedSpotEntry(val) => {
                SubmittedSpotEntry::cairo_serialized_size(val) + 1
            }
            _ => 0,
        }
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        match __rust {
            Event::SubmittedSpotEntry(val) => {
                let mut temp = vec![];
                temp.extend(usize::cairo_serialize(&0usize));
                temp.extend(SubmittedSpotEntry::cairo_serialize(val));
                temp
            }
            _ => vec![],
        }
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let __f = __felts[__offset];
        let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());
        match __index as usize {
            0usize => {
                Ok(
                    Event::SubmittedSpotEntry(
                        SubmittedSpotEntry::cairo_deserialize(__felts, __offset + 1)?,
                    ),
                )
            }
            _ => {
                return Err(
                    cainome::cairo_serde::Error::Deserialize(
                        format!("Index not handle for enum {}", "Event"),
                    ),
                );
            }
        }
    }
}
impl TryFrom<starknet::core::types::EmittedEvent> for Event {
    type Error = String;
    fn try_from(
        event: starknet::core::types::EmittedEvent,
    ) -> Result<Self, Self::Error> {
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty() {
            return Err("Event has no key".to_string());
        }
        let selector = event.keys[0];
        if selector
            == starknet::core::utils::get_selector_from_name("SubmittedSpotEntry")
                .unwrap_or_else(|_| {
                    panic!("Invalid selector for {}", "SubmittedSpotEntry")
                })
        {
            let mut key_offset = 0 + 1;
            let mut data_offset = 0;
            let spot_entry = match SpotEntry::cairo_deserialize(
                &event.data,
                data_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}", "spot_entry",
                            "SubmittedSpotEntry", e
                        ),
                    );
                }
            };
            data_offset += SpotEntry::cairo_serialized_size(&spot_entry);
            return Ok(Event::SubmittedSpotEntry(SubmittedSpotEntry { spot_entry }));
        }
        Err(format!("Could not match any event from keys {:?}", event.keys))
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub enum DataType {
    SpotEntry(starknet::core::types::Felt),
    FutureEntry((starknet::core::types::Felt, u64)),
    GenericEntry(starknet::core::types::Felt),
}
impl cainome::cairo_serde::CairoSerde for DataType {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = std::option::Option::None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        match __rust {
            DataType::SpotEntry(val) => {
                starknet::core::types::Felt::cairo_serialized_size(val) + 1
            }
            DataType::FutureEntry(val) => {
                <(starknet::core::types::Felt, u64)>::cairo_serialized_size(val) + 1
            }
            DataType::GenericEntry(val) => {
                starknet::core::types::Felt::cairo_serialized_size(val) + 1
            }
            _ => 0,
        }
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        match __rust {
            DataType::SpotEntry(val) => {
                let mut temp = vec![];
                temp.extend(usize::cairo_serialize(&0usize));
                temp.extend(starknet::core::types::Felt::cairo_serialize(val));
                temp
            }
            DataType::FutureEntry(val) => {
                let mut temp = vec![];
                temp.extend(usize::cairo_serialize(&1usize));
                temp.extend(<(starknet::core::types::Felt, u64)>::cairo_serialize(val));
                temp
            }
            DataType::GenericEntry(val) => {
                let mut temp = vec![];
                temp.extend(usize::cairo_serialize(&2usize));
                temp.extend(starknet::core::types::Felt::cairo_serialize(val));
                temp
            }
            _ => vec![],
        }
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let __f = __felts[__offset];
        let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());
        match __index as usize {
            0usize => {
                Ok(
                    DataType::SpotEntry(
                        starknet::core::types::Felt::cairo_deserialize(
                            __felts,
                            __offset + 1,
                        )?,
                    ),
                )
            }
            1usize => {
                Ok(
                    DataType::FutureEntry(
                        <(
                            starknet::core::types::Felt,
                            u64,
                        )>::cairo_deserialize(__felts, __offset + 1)?,
                    ),
                )
            }
            2usize => {
                Ok(
                    DataType::GenericEntry(
                        starknet::core::types::Felt::cairo_deserialize(
                            __felts,
                            __offset + 1,
                        )?,
                    ),
                )
            }
            _ => {
                return Err(
                    cainome::cairo_serde::Error::Deserialize(
                        format!("Index not handle for enum {}", "DataType"),
                    ),
                );
            }
        }
    }
}
impl<A: starknet::accounts::ConnectedAccount + Sync> MockPragmaOracle<A> {
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_data_median_getcall(
        &self,
        data_type: &DataType,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(DataType::cairo_serialize(data_type));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("get_data_median"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_data_median(
        &self,
        data_type: &DataType,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(DataType::cairo_serialize(data_type));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("get_data_median"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_num_sources_aggregated_getcall(
        &self,
        key: &starknet::core::types::Felt,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("get_num_sources_aggregated"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_num_sources_aggregated(
        &self,
        key: &starknet::core::types::Felt,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("get_num_sources_aggregated"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_last_updated_timestamp_getcall(
        &self,
        key: &starknet::core::types::Felt,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("get_last_updated_timestamp"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_last_updated_timestamp(
        &self,
        key: &starknet::core::types::Felt,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("get_last_updated_timestamp"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_price_getcall(
        &self,
        key: &starknet::core::types::Felt,
        price: &u128,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        __calldata.extend(u128::cairo_serialize(price));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("set_price"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_price(
        &self,
        key: &starknet::core::types::Felt,
        price: &u128,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        __calldata.extend(u128::cairo_serialize(price));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("set_price"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_num_sources_aggregated_getcall(
        &self,
        key: &starknet::core::types::Felt,
        num_sources_aggregated: &u32,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        __calldata.extend(u32::cairo_serialize(num_sources_aggregated));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("set_num_sources_aggregated"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_num_sources_aggregated(
        &self,
        key: &starknet::core::types::Felt,
        num_sources_aggregated: &u32,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        __calldata.extend(u32::cairo_serialize(num_sources_aggregated));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("set_num_sources_aggregated"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_last_updated_timestamp_getcall(
        &self,
        key: &starknet::core::types::Felt,
        last_updated_timestamp: &u64,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        __calldata.extend(u64::cairo_serialize(last_updated_timestamp));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("set_last_updated_timestamp"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_last_updated_timestamp(
        &self,
        key: &starknet::core::types::Felt,
        last_updated_timestamp: &u64,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(starknet::core::types::Felt::cairo_serialize(key));
        __calldata.extend(u64::cairo_serialize(last_updated_timestamp));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("set_last_updated_timestamp"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
}
impl<P: starknet::providers::Provider + Sync> MockPragmaOracleReader<P> {}
