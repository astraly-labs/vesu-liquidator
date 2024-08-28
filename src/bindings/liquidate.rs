#[derive(Debug)]
pub struct Liquidate<A: starknet::accounts::ConnectedAccount + Sync> {
    pub address: starknet::core::types::Felt,
    pub account: A,
    pub block_id: starknet::core::types::BlockId,
}
impl<A: starknet::accounts::ConnectedAccount + Sync> Liquidate<A> {
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
pub struct LiquidateReader<P: starknet::providers::Provider + Sync> {
    pub address: starknet::core::types::Felt,
    pub provider: P,
    pub block_id: starknet::core::types::BlockId,
}
impl<P: starknet::providers::Provider + Sync> LiquidateReader<P> {
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
pub struct LiquidateParams {
    pub pool_id: starknet::core::types::Felt,
    pub collateral_asset: cainome::cairo_serde::ContractAddress,
    pub debt_asset: cainome::cairo_serde::ContractAddress,
    pub user: cainome::cairo_serde::ContractAddress,
    pub recipient: cainome::cairo_serde::ContractAddress,
    pub min_collateral_to_receive: cainome::cairo_serde::U256,
    pub full_liquidation: bool,
    pub liquidate_swap: Swap,
    pub withdraw_swap: Swap,
}
impl cainome::cairo_serde::CairoSerde for LiquidateParams {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += starknet::core::types::Felt::cairo_serialized_size(&__rust.pool_id);
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.collateral_asset,
            );
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.debt_asset,
            );
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.user,
            );
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.recipient,
            );
        __size
            += cainome::cairo_serde::U256::cairo_serialized_size(
                &__rust.min_collateral_to_receive,
            );
        __size += bool::cairo_serialized_size(&__rust.full_liquidation);
        __size += Swap::cairo_serialized_size(&__rust.liquidate_swap);
        __size += Swap::cairo_serialized_size(&__rust.withdraw_swap);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(starknet::core::types::Felt::cairo_serialize(&__rust.pool_id));
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(
                    &__rust.collateral_asset,
                ),
            );
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(
                    &__rust.debt_asset,
                ),
            );
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.user),
            );
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.recipient),
            );
        __out
            .extend(
                cainome::cairo_serde::U256::cairo_serialize(
                    &__rust.min_collateral_to_receive,
                ),
            );
        __out.extend(bool::cairo_serialize(&__rust.full_liquidation));
        __out.extend(Swap::cairo_serialize(&__rust.liquidate_swap));
        __out.extend(Swap::cairo_serialize(&__rust.withdraw_swap));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let pool_id = starknet::core::types::Felt::cairo_deserialize(__felts, __offset)?;
        __offset += starknet::core::types::Felt::cairo_serialized_size(&pool_id);
        let collateral_asset = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &collateral_asset,
            );
        let debt_asset = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&debt_asset);
        let user = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&user);
        let recipient = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&recipient);
        let min_collateral_to_receive = cainome::cairo_serde::U256::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::U256::cairo_serialized_size(
                &min_collateral_to_receive,
            );
        let full_liquidation = bool::cairo_deserialize(__felts, __offset)?;
        __offset += bool::cairo_serialized_size(&full_liquidation);
        let liquidate_swap = Swap::cairo_deserialize(__felts, __offset)?;
        __offset += Swap::cairo_serialized_size(&liquidate_swap);
        let withdraw_swap = Swap::cairo_deserialize(__felts, __offset)?;
        __offset += Swap::cairo_serialized_size(&withdraw_swap);
        Ok(LiquidateParams {
            pool_id,
            collateral_asset,
            debt_asset,
            user,
            recipient,
            min_collateral_to_receive,
            full_liquidation,
            liquidate_swap,
            withdraw_swap,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouteNode {
    pub pool_key: PoolKey,
    pub sqrt_ratio_limit: cainome::cairo_serde::U256,
    pub skip_ahead: u128,
}
impl cainome::cairo_serde::CairoSerde for RouteNode {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += PoolKey::cairo_serialized_size(&__rust.pool_key);
        __size
            += cainome::cairo_serde::U256::cairo_serialized_size(
                &__rust.sqrt_ratio_limit,
            );
        __size += u128::cairo_serialized_size(&__rust.skip_ahead);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(PoolKey::cairo_serialize(&__rust.pool_key));
        __out
            .extend(
                cainome::cairo_serde::U256::cairo_serialize(&__rust.sqrt_ratio_limit),
            );
        __out.extend(u128::cairo_serialize(&__rust.skip_ahead));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let pool_key = PoolKey::cairo_deserialize(__felts, __offset)?;
        __offset += PoolKey::cairo_serialized_size(&pool_key);
        let sqrt_ratio_limit = cainome::cairo_serde::U256::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += cainome::cairo_serde::U256::cairo_serialized_size(&sqrt_ratio_limit);
        let skip_ahead = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&skip_ahead);
        Ok(RouteNode {
            pool_key,
            sqrt_ratio_limit,
            skip_ahead,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct ICoreDispatcher {
    pub contract_address: cainome::cairo_serde::ContractAddress,
}
impl cainome::cairo_serde::CairoSerde for ICoreDispatcher {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.contract_address,
            );
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(
                    &__rust.contract_address,
                ),
            );
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let contract_address = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &contract_address,
            );
        Ok(ICoreDispatcher {
            contract_address,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct ISingletonDispatcher {
    pub contract_address: cainome::cairo_serde::ContractAddress,
}
impl cainome::cairo_serde::CairoSerde for ISingletonDispatcher {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.contract_address,
            );
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(
                    &__rust.contract_address,
                ),
            );
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let contract_address = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &contract_address,
            );
        Ok(ISingletonDispatcher {
            contract_address,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct Swap {
    pub route: Vec<RouteNode>,
    pub token_amount: TokenAmount,
    pub limit_amount: u128,
}
impl cainome::cairo_serde::CairoSerde for Swap {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += Vec::<RouteNode>::cairo_serialized_size(&__rust.route);
        __size += TokenAmount::cairo_serialized_size(&__rust.token_amount);
        __size += u128::cairo_serialized_size(&__rust.limit_amount);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(Vec::<RouteNode>::cairo_serialize(&__rust.route));
        __out.extend(TokenAmount::cairo_serialize(&__rust.token_amount));
        __out.extend(u128::cairo_serialize(&__rust.limit_amount));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let route = Vec::<RouteNode>::cairo_deserialize(__felts, __offset)?;
        __offset += Vec::<RouteNode>::cairo_serialized_size(&route);
        let token_amount = TokenAmount::cairo_deserialize(__felts, __offset)?;
        __offset += TokenAmount::cairo_serialized_size(&token_amount);
        let limit_amount = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&limit_amount);
        Ok(Swap {
            route,
            token_amount,
            limit_amount,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiquidatePosition {
    pub pool_id: starknet::core::types::Felt,
    pub collateral_asset: cainome::cairo_serde::ContractAddress,
    pub debt_asset: cainome::cairo_serde::ContractAddress,
    pub user: cainome::cairo_serde::ContractAddress,
    pub residual: cainome::cairo_serde::U256,
    pub collateral_delta: cainome::cairo_serde::U256,
    pub debt_delta: cainome::cairo_serde::U256,
}
impl cainome::cairo_serde::CairoSerde for LiquidatePosition {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += starknet::core::types::Felt::cairo_serialized_size(&__rust.pool_id);
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.collateral_asset,
            );
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.debt_asset,
            );
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.user,
            );
        __size += cainome::cairo_serde::U256::cairo_serialized_size(&__rust.residual);
        __size
            += cainome::cairo_serde::U256::cairo_serialized_size(
                &__rust.collateral_delta,
            );
        __size += cainome::cairo_serde::U256::cairo_serialized_size(&__rust.debt_delta);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(starknet::core::types::Felt::cairo_serialize(&__rust.pool_id));
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(
                    &__rust.collateral_asset,
                ),
            );
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(
                    &__rust.debt_asset,
                ),
            );
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.user),
            );
        __out.extend(cainome::cairo_serde::U256::cairo_serialize(&__rust.residual));
        __out
            .extend(
                cainome::cairo_serde::U256::cairo_serialize(&__rust.collateral_delta),
            );
        __out.extend(cainome::cairo_serde::U256::cairo_serialize(&__rust.debt_delta));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let pool_id = starknet::core::types::Felt::cairo_deserialize(__felts, __offset)?;
        __offset += starknet::core::types::Felt::cairo_serialized_size(&pool_id);
        let collateral_asset = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &collateral_asset,
            );
        let debt_asset = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&debt_asset);
        let user = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&user);
        let residual = cainome::cairo_serde::U256::cairo_deserialize(__felts, __offset)?;
        __offset += cainome::cairo_serde::U256::cairo_serialized_size(&residual);
        let collateral_delta = cainome::cairo_serde::U256::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += cainome::cairo_serde::U256::cairo_serialized_size(&collateral_delta);
        let debt_delta = cainome::cairo_serde::U256::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += cainome::cairo_serde::U256::cairo_serialized_size(&debt_delta);
        Ok(LiquidatePosition {
            pool_id,
            collateral_asset,
            debt_asset,
            user,
            residual,
            collateral_delta,
            debt_delta,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenAmount {
    pub token: cainome::cairo_serde::ContractAddress,
    pub amount: I129,
}
impl cainome::cairo_serde::CairoSerde for TokenAmount {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.token,
            );
        __size += I129::cairo_serialized_size(&__rust.amount);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.token),
            );
        __out.extend(I129::cairo_serialize(&__rust.amount));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let token = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&token);
        let amount = I129::cairo_deserialize(__felts, __offset)?;
        __offset += I129::cairo_serialized_size(&amount);
        Ok(TokenAmount { token, amount })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolKey {
    pub token0: cainome::cairo_serde::ContractAddress,
    pub token1: cainome::cairo_serde::ContractAddress,
    pub fee: u128,
    pub tick_spacing: u128,
    pub extension: cainome::cairo_serde::ContractAddress,
}
impl cainome::cairo_serde::CairoSerde for PoolKey {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.token0,
            );
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.token1,
            );
        __size += u128::cairo_serialized_size(&__rust.fee);
        __size += u128::cairo_serialized_size(&__rust.tick_spacing);
        __size
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                &__rust.extension,
            );
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.token0),
            );
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.token1),
            );
        __out.extend(u128::cairo_serialize(&__rust.fee));
        __out.extend(u128::cairo_serialize(&__rust.tick_spacing));
        __out
            .extend(
                cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.extension),
            );
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let token0 = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&token0);
        let token1 = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&token1);
        let fee = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&fee);
        let tick_spacing = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&tick_spacing);
        let extension = cainome::cairo_serde::ContractAddress::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&extension);
        Ok(PoolKey {
            token0,
            token1,
            fee,
            tick_spacing,
            extension,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct I129 {
    pub mag: u128,
    pub sign: bool,
}
impl cainome::cairo_serde::CairoSerde for I129 {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += u128::cairo_serialized_size(&__rust.mag);
        __size += bool::cairo_serialized_size(&__rust.sign);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out.extend(u128::cairo_serialize(&__rust.mag));
        __out.extend(bool::cairo_serialize(&__rust.sign));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let mag = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&mag);
        let sign = bool::cairo_deserialize(__felts, __offset)?;
        __offset += bool::cairo_serialized_size(&sign);
        Ok(I129 { mag, sign })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiquidateResponse {
    pub liquidated_collateral: cainome::cairo_serde::U256,
    pub repaid_debt: cainome::cairo_serde::U256,
    pub residual_collateral: cainome::cairo_serde::U256,
}
impl cainome::cairo_serde::CairoSerde for LiquidateResponse {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size
            += cainome::cairo_serde::U256::cairo_serialized_size(
                &__rust.liquidated_collateral,
            );
        __size += cainome::cairo_serde::U256::cairo_serialized_size(&__rust.repaid_debt);
        __size
            += cainome::cairo_serde::U256::cairo_serialized_size(
                &__rust.residual_collateral,
            );
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        let mut __out: Vec<starknet::core::types::Felt> = vec![];
        __out
            .extend(
                cainome::cairo_serde::U256::cairo_serialize(
                    &__rust.liquidated_collateral,
                ),
            );
        __out.extend(cainome::cairo_serde::U256::cairo_serialize(&__rust.repaid_debt));
        __out
            .extend(
                cainome::cairo_serde::U256::cairo_serialize(&__rust.residual_collateral),
            );
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let mut __offset = __offset;
        let liquidated_collateral = cainome::cairo_serde::U256::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::U256::cairo_serialized_size(&liquidated_collateral);
        let repaid_debt = cainome::cairo_serde::U256::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset += cainome::cairo_serde::U256::cairo_serialized_size(&repaid_debt);
        let residual_collateral = cainome::cairo_serde::U256::cairo_deserialize(
            __felts,
            __offset,
        )?;
        __offset
            += cainome::cairo_serde::U256::cairo_serialized_size(&residual_collateral);
        Ok(LiquidateResponse {
            liquidated_collateral,
            repaid_debt,
            residual_collateral,
        })
    }
}
#[derive(Debug, PartialEq, PartialOrd, Clone, serde::Serialize, serde::Deserialize)]
pub enum Event {
    LiquidatePosition(LiquidatePosition),
}
impl cainome::cairo_serde::CairoSerde for Event {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = std::option::Option::None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        match __rust {
            Event::LiquidatePosition(val) => {
                LiquidatePosition::cairo_serialized_size(val) + 1
            }
            _ => 0,
        }
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        match __rust {
            Event::LiquidatePosition(val) => {
                let mut temp = vec![];
                temp.extend(usize::cairo_serialize(&0usize));
                temp.extend(LiquidatePosition::cairo_serialize(val));
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
                    Event::LiquidatePosition(
                        LiquidatePosition::cairo_deserialize(__felts, __offset + 1)?,
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
            == starknet::core::utils::get_selector_from_name("LiquidatePosition")
                .unwrap_or_else(|_| {
                    panic!("Invalid selector for {}", "LiquidatePosition")
                })
        {
            let mut key_offset = 0 + 1;
            let mut data_offset = 0;
            let pool_id = match starknet::core::types::Felt::cairo_deserialize(
                &event.keys,
                key_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}",
                            "pool_id",
                            "LiquidatePosition",
                            e,
                        ),
                    );
                }
            };
            key_offset += starknet::core::types::Felt::cairo_serialized_size(&pool_id);
            let collateral_asset = match cainome::cairo_serde::ContractAddress::cairo_deserialize(
                &event.keys,
                key_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}",
                            "collateral_asset",
                            "LiquidatePosition",
                            e,
                        ),
                    );
                }
            };
            key_offset
                += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                    &collateral_asset,
                );
            let debt_asset = match cainome::cairo_serde::ContractAddress::cairo_deserialize(
                &event.keys,
                key_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}",
                            "debt_asset",
                            "LiquidatePosition",
                            e,
                        ),
                    );
                }
            };
            key_offset
                += cainome::cairo_serde::ContractAddress::cairo_serialized_size(
                    &debt_asset,
                );
            let user = match cainome::cairo_serde::ContractAddress::cairo_deserialize(
                &event.keys,
                key_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}",
                            "user",
                            "LiquidatePosition",
                            e,
                        ),
                    );
                }
            };
            key_offset
                += cainome::cairo_serde::ContractAddress::cairo_serialized_size(&user);
            let residual = match cainome::cairo_serde::U256::cairo_deserialize(
                &event.data,
                data_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}",
                            "residual",
                            "LiquidatePosition",
                            e,
                        ),
                    );
                }
            };
            data_offset += cainome::cairo_serde::U256::cairo_serialized_size(&residual);
            let collateral_delta = match cainome::cairo_serde::U256::cairo_deserialize(
                &event.data,
                data_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}",
                            "collateral_delta",
                            "LiquidatePosition",
                            e,
                        ),
                    );
                }
            };
            data_offset
                += cainome::cairo_serde::U256::cairo_serialized_size(&collateral_delta);
            let debt_delta = match cainome::cairo_serde::U256::cairo_deserialize(
                &event.data,
                data_offset,
            ) {
                Ok(v) => v,
                Err(e) => {
                    return Err(
                        format!(
                            "Could not deserialize field {} for {}: {:?}",
                            "debt_delta",
                            "LiquidatePosition",
                            e,
                        ),
                    );
                }
            };
            data_offset
                += cainome::cairo_serde::U256::cairo_serialized_size(&debt_delta);
            return Ok(
                Event::LiquidatePosition(LiquidatePosition {
                    pool_id,
                    collateral_asset,
                    debt_asset,
                    user,
                    residual,
                    collateral_delta,
                    debt_delta,
                }),
            );
        }
        Err(format!("Could not match any event from keys {:?}", event.keys))
    }
}
impl<A: starknet::accounts::ConnectedAccount + Sync> Liquidate<A> {
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn locked_getcall(
        &self,
        id: &u32,
        data: &Vec<starknet::core::types::Felt>,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(u32::cairo_serialize(id));
        __calldata.extend(Vec::<starknet::core::types::Felt>::cairo_serialize(data));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("locked"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn locked(
        &self,
        id: &u32,
        data: &Vec<starknet::core::types::Felt>,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(u32::cairo_serialize(id));
        __calldata.extend(Vec::<starknet::core::types::Felt>::cairo_serialize(data));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("locked"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn liquidate_getcall(
        &self,
        params: &LiquidateParams,
    ) -> starknet::accounts::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(LiquidateParams::cairo_serialize(params));
        starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("liquidate"),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn liquidate(
        &self,
        params: &LiquidateParams,
    ) -> starknet::accounts::ExecutionV1<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = vec![];
        __calldata.extend(LiquidateParams::cairo_serialize(params));
        let __call = starknet::accounts::Call {
            to: self.address,
            selector: starknet::macros::selector!("liquidate"),
            calldata: __calldata,
        };
        self.account.execute_v1(vec![__call])
    }
}
impl<P: starknet::providers::Provider + Sync> LiquidateReader<P> {}
