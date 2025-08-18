#![feature(core_intrinsics)]

use base64ct::{Base64, Encoding};
use sui_graphql_client::Client as GraphQLClient;
use sui_rpc::client::AuthInterceptor;
use sui_rpc::field::FieldMask;
use sui_rpc::proto::sui::rpc::v2beta2::ledger_service_client::LedgerServiceClient;
use sui_rpc::proto::sui::rpc::v2beta2::live_data_service_client::LiveDataServiceClient;
use sui_rpc::proto::sui::rpc::v2beta2::GetObjectRequest;
use sui_rpc::proto::sui::rpc::v2beta2::ListDynamicFieldsRequest;
use sui_rpc::proto::sui::rpc::v2beta2::Object;
use sui_sdk_types::Address;
use sui_sdk_types::TypeTag;
use sui_sdk_types::StructTag;
use std::str::FromStr;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;

pub mod common;
pub mod pools;
pub mod positions;
pub mod strategies;
pub mod types;

pub use types::{SupportedDex, *};

const BAG_OBJECT_ID: &str = "0x47f27839b9cbb864bf9a93223eb7c97aee04788fc2603edf56200909aa672ca8";

/// Helper function để format coin type string với prefix 0x nếu cần thiết
fn format_coin_type(coin_type: &str) -> String {
    if coin_type.starts_with("0x") {
        coin_type.to_string()
    } else {
        // Tìm vị trí của dấu :: đầu tiên
        if let Some(pos) = coin_type.find("::") {
            let (address_part, rest) = coin_type.split_at(pos);
            format!("0x{}{}", address_part, rest)
        } else {
            coin_type.to_string()
        }
    }
}

pub async fn into_processed_pool_data<'a>(
    graphql_client: &'a mut GraphQLClient,
    pool_object: Object,
    strategy_object: Object,
) -> Result<ProcessedPoolData, anyhow::Error> {
    let pool_json = pool_object.json.unwrap();
    let strategy_json = strategy_object.json.unwrap();

    let pool_data = pools::try_match(pool_json)?;
    let strategy_data = strategies::try_match(strategy_json)?;

    let position_registry_id = match &strategy_data {
        Strategy::AutoRebalance(auto_rebalance) => auto_rebalance.position_registry_id,
        _ => return Err(anyhow::anyhow!("Unknown strategy type")),
    };
    
    // filter out
    let position_field = graphql_client
        .dynamic_field(
            Address::from_hex(BAG_OBJECT_ID)
                .unwrap(),
            TypeTag::U64,
            position_registry_id,
        )
        .await?
        .unwrap()
        .value_as_json
        .unwrap();

    let position_json = serde_json::to_value(&position_field)?;
    let position_data = positions::try_match(Box::new(position_json))?;

    let (current_tick, current_sqrt_price, tick_spacing, dex) =
        match pool_data {
            Pool::Cetus(cetus_pool) => (
                cetus_pool.current_tick_index.bits.parse::<u32>()?,
                cetus_pool.current_sqrt_price.parse::<u128>()?,
                cetus_pool.tick_spacing as u32,
                SupportedDex::Cetus,
            ),
        };

    let (tick_lower, tick_upper, coin_a_type, coin_b_type) = match position_data {
        Position::Cetus(cetus_position) => {
            let formatted_coin_a = format_coin_type(&cetus_position.coin_type_a.name);
            let formatted_coin_b = format_coin_type(&cetus_position.coin_type_b.name);
            (
                cetus_position.tick_lower_index.bits.parse::<u32>()?,
                cetus_position.tick_upper_index.bits.parse::<u32>()?,
                TypeTag::Struct(Box::new(StructTag::from_str(&formatted_coin_a)?)),
                TypeTag::Struct(Box::new(StructTag::from_str(&formatted_coin_b)?)),
            )
        },
    };

    // Create request based on strategy type and pool type
    let request = match &strategy_data {
        Strategy::AutoRebalance(_auto_rebalance) => Request::Rebalance(RebalanceRequest {
            strategy_id: Address::from_hex(strategy_object.object_id.unwrap_or_default()).unwrap(),
            current_tick_u32: current_tick as u32,
            current_sqrt_price: current_sqrt_price,
            tick_spacing: tick_spacing,
            tick_lower_index_u32: tick_lower as u32,
            tick_upper_index_u32: tick_upper as u32,
        }),
        _ => return Err(anyhow::anyhow!("Unknown strategy type")),
    };

    println!("request: {:?}", request);

    let auto_rebalance_strategy = match &strategy_data {
        Strategy::AutoRebalance(auto_rebalance) => Some(auto_rebalance.clone()),
        _ => None,
    };

    Ok(ProcessedPoolData {
        auto_rebalance_strategy,
        request,
        dex,
        position_registry_id,
        coin_a_type,
        coin_b_type,
    })
}
