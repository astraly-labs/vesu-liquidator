use starknet_rust::core::types::{Call, Felt, U256};
use starknet_rust::macros::selector;

// ---- Cairo types used by the Vesu Liquidate contract ----

#[derive(Debug, Clone, Default)]
pub struct I129 {
    pub mag: u128,
    pub sign: bool,
}

impl I129 {
    fn to_calldata(&self) -> Vec<Felt> {
        vec![Felt::from(self.mag), Felt::from(self.sign as u64)]
    }
}

#[derive(Debug, Clone)]
pub struct PoolKey {
    pub token0: Felt,
    pub token1: Felt,
    pub fee: u128,
    pub tick_spacing: u128,
    pub extension: Felt,
}

impl PoolKey {
    fn to_calldata(&self) -> Vec<Felt> {
        vec![
            self.token0,
            self.token1,
            Felt::from(self.fee),
            Felt::from(self.tick_spacing),
            self.extension,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct RouteNode {
    pub pool_key: PoolKey,
    pub sqrt_ratio_limit: U256,
    pub skip_ahead: u128,
}

impl RouteNode {
    fn to_calldata(&self) -> Vec<Felt> {
        let mut out = self.pool_key.to_calldata();
        out.push(Felt::from(self.sqrt_ratio_limit.low()));
        out.push(Felt::from(self.sqrt_ratio_limit.high()));
        out.push(Felt::from(self.skip_ahead));
        out
    }
}

#[derive(Debug, Clone)]
pub struct TokenAmount {
    pub token: Felt,
    pub amount: I129,
}

impl TokenAmount {
    fn to_calldata(&self) -> Vec<Felt> {
        let mut out = vec![self.token];
        out.extend(self.amount.to_calldata());
        out
    }
}

#[derive(Debug, Clone)]
pub struct Swap {
    pub route: Vec<RouteNode>,
    pub token_amount: TokenAmount,
}

impl Swap {
    fn to_calldata(&self) -> Vec<Felt> {
        let mut out = vec![Felt::from(self.route.len() as u64)];
        for node in &self.route {
            out.extend(node.to_calldata());
        }
        out.extend(self.token_amount.to_calldata());
        out
    }
}

#[derive(Debug, Clone)]
pub struct LiquidateParams {
    pub pool_id: Felt,
    pub collateral_asset: Felt,
    pub debt_asset: Felt,
    pub user: Felt,
    pub recipient: Felt,
    pub min_collateral_to_receive: U256,
    pub debt_to_repay: U256,
    pub liquidate_swap: Vec<Swap>,
    pub liquidate_swap_limit_amount: u128,
    pub liquidate_swap_weights: Vec<u128>,
    pub withdraw_swap: Vec<Swap>,
    pub withdraw_swap_limit_amount: u128,
    pub withdraw_swap_weights: Vec<u128>,
}

impl LiquidateParams {
    pub fn to_calldata(&self) -> Vec<Felt> {
        let mut out = vec![
            self.pool_id,
            self.collateral_asset,
            self.debt_asset,
            self.user,
            self.recipient,
            Felt::from(self.min_collateral_to_receive.low()),
            Felt::from(self.min_collateral_to_receive.high()),
            Felt::from(self.debt_to_repay.low()),
            Felt::from(self.debt_to_repay.high()),
        ];

        // liquidate_swap: Span<Swap>
        out.push(Felt::from(self.liquidate_swap.len() as u64));
        for swap in &self.liquidate_swap {
            out.extend(swap.to_calldata());
        }

        out.push(Felt::from(self.liquidate_swap_limit_amount));

        // liquidate_swap_weights: Span<u128>
        out.push(Felt::from(self.liquidate_swap_weights.len() as u64));
        for w in &self.liquidate_swap_weights {
            out.push(Felt::from(*w));
        }

        // withdraw_swap: Span<Swap>
        out.push(Felt::from(self.withdraw_swap.len() as u64));
        for swap in &self.withdraw_swap {
            out.extend(swap.to_calldata());
        }

        out.push(Felt::from(self.withdraw_swap_limit_amount));

        // withdraw_swap_weights: Span<u128>
        out.push(Felt::from(self.withdraw_swap_weights.len() as u64));
        for w in &self.withdraw_swap_weights {
            out.push(Felt::from(*w));
        }

        out
    }
}

/// Build a `Call` to the liquidate entrypoint on the given contract.
pub fn build_liquidate_call(contract_address: Felt, params: &LiquidateParams) -> Call {
    Call {
        to: contract_address,
        selector: selector!("liquidate"),
        calldata: params.to_calldata(),
    }
}
