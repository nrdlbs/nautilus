use serde::{Deserialize, Serialize};
use sui_sdk_types::TypeTag;

// ============================================================================
// REQUEST TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RebalanceRequest {
    pub strategy_id: String,
    pub current_tick_u32: u32,
    pub current_sqrt_price: u128,
    pub tick_spacing: u32,
    pub tick_lower_index_u32: u32,
    pub tick_upper_index_u32: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompoundRequest {
    pub strategy_id: String,
    pub pending_rewards_value: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Request {
    Rebalance(RebalanceRequest),
    Compound(CompoundRequest),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessedPoolData {
    pub request: Request,
    pub auto_rebalance_strategy: Option<AutoRebalanceStrategy>,
    pub dex: SupportedDex,
    pub position_registry_id: u64,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
}

// ============================================================================
// POOL TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CetusPoolData {
    pub coin_a: String,
    pub coin_b: String,
    pub current_sqrt_price: String,
    pub current_tick_index: TickIndex,
    pub fee_growth_global_a: String,
    pub fee_growth_global_b: String,
    pub fee_protocol_coin_a: String,
    pub fee_protocol_coin_b: String,
    pub fee_rate: String,
    pub id: String,
    pub index: String,
    pub is_pause: bool,
    pub liquidity: String,
    pub position_manager: PositionManager,
    pub rewarder_manager: RewarderManager,
    pub tick_manager: TickManager,
    pub tick_spacing: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TickIndex {
    pub bits: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionManager {
    pub position_index: String,
    pub positions: PositionsList,
    pub tick_spacing: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionsList {
    pub head: String,
    pub id: String,
    pub size: String,
    pub tail: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RewarderManager {
    pub last_updated_time: String,
    pub points_growth_global: String,
    pub points_released: String,
    pub rewarders: Vec<Rewarder>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rewarder {
    pub emissions_per_second: String,
    pub growth_global: String,
    pub reward_coin: RewardCoin,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RewardCoin {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TickManager {
    pub tick_spacing: u64,
    pub ticks: TicksData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TicksData {
    pub head: Vec<TickOption>,
    pub id: String,
    pub level: String,
    pub list_p: String,
    pub max_level: String,
    pub random: RandomSeed,
    pub size: String,
    pub tail: TickOption,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TickOption {
    pub is_none: bool,
    pub v: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RandomSeed {
    pub seed: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Pool {
    Cetus(CetusPoolData),
}

// ============================================================================
// POSITION TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CetusPosition {
    pub id: String,
    pub pool: String,
    pub index: u64,
    pub coin_type_a: TypeName,
    pub coin_type_b: TypeName,
    pub name: String,
    pub description: String,
    pub url: String,
    pub tick_lower_index: I32Wrapper,
    pub tick_upper_index: I32Wrapper,
    pub liquidity: u128,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeName {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Position {
    Cetus(CetusPosition),
}

// ============================================================================
// STRATEGY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoRebalanceStrategy {
    pub id: String,
    pub version: u64,
    pub owner: String,
    pub position_registry_id: u64,
    pub description: String,
    pub lower_sqrt_price_change_threshold_bps: u64, // 10000 = 100%
    pub upper_sqrt_price_change_threshold_bps: u64, // 10000 = 100%
    pub lower_sqrt_price_change_threshold_direction: bool, // true = in range, false = out of range
    pub upper_sqrt_price_change_threshold_direction: bool, // true = in range, false = out of range
    pub rebalance_cooldown_secs: u64,
    pub range_multiplier: u64,
    pub rebalance_max_tick: I32Wrapper,
    pub rebalance_min_tick: I32Wrapper,
    pub rebalance_paused: bool,
    pub lp_slippage_tolerance_bps: u64,
    // state
    pub last_rebalance_timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Strategy {
    AutoRebalance(AutoRebalanceStrategy),
}

// ============================================================================
// COMMON TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct I32Wrapper {
    pub bits: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupportedDex {
    MMT,
    Bluefin,
    Cetus,
    FlowX
}