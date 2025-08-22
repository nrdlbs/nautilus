use crate::{
    parsers::{BalancesBag, CompoundRequest, RebalanceRequest, SupportedDex},
    transactions_builder::constant::{
        CLOCK_OBJECT_ID, GLOBAL_CONFIG_OBJECT_ID, INTEGER_MATE_PACKAGE_ID, KURAGE_PACKAGE_ID, REGISTRY_OBJECT_ID
    },
};
use std::str::FromStr;
use sui_graphql_client::Client;
use sui_sdk_types::{Address, Identifier, StructTag, TypeTag};
use sui_transaction_builder::{Function, TransactionBuilder};
pub mod argument;
pub mod cetus;
pub mod constant;
pub mod helper;
pub mod swap;

// array of position types
pub fn get_position_types(dex: SupportedDex) -> &'static str {
    match dex {
        SupportedDex::Cetus => {
            "0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb::position::Position"
        }
        _ => panic!("Unsupported dex"),
    }
}

pub struct DexTransactionBuilder<'a> {
    tx: TransactionBuilder,
    client: &'a Client,
    arg_cache: argument::ArgCache,
}

impl<'a> DexTransactionBuilder<'a> {
    pub async fn new(client: &'a Client, caller: Address, gas_budget: u64) -> Self {
        let tx = helper::new_with_gas(client, caller, gas_budget)
            .await
            .unwrap();
        Self {
            tx,
            client,
            arg_cache: argument::ArgCache::default(),
        }
    }

    pub async fn rebalance(
        mut self,
        request: RebalanceRequest,
        pool_id: String,
        coin_a_type: TypeTag,
        coin_b_type: TypeTag,
        new_tick_lower_index: u32,
        new_tick_upper_index: u32,
        liquidity: u128,
        position_registry_id: u64,
        dex: SupportedDex,
        enclave_id: String,
        signature: Vec<u8>,
        timestamp_ms: u64,
        lp_slippage_tolerance_bps: u64,
        balances_bag: BalancesBag,
        rewarder_coin_types: Vec<TypeTag>,
    ) -> TransactionBuilder {
        let pos_type = get_position_types(dex);

        let global_config_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            &mut self.arg_cache,
            Address::from_hex(GLOBAL_CONFIG_OBJECT_ID).unwrap(),
        )
        .await
        .unwrap();
        let registry_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            &mut self.arg_cache,
            Address::from_hex(REGISTRY_OBJECT_ID).unwrap(),
        )
        .await
        .unwrap();
        let strategy_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            &mut self.arg_cache,
            request.strategy_id.clone(),
        )
        .await
        .unwrap();
        let enclave_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            &mut self.arg_cache,
            Address::from_hex(&enclave_id).unwrap(),
        )
        .await
        .unwrap();
        let timestamp_arg = argument::pure(&mut self.tx, timestamp_ms).unwrap();
        let signature_arg = argument::pure(&mut self.tx, signature).unwrap();
        let clock_arg = argument::shared_ref_cached(
            self.client,
            &mut self.tx,
            &mut self.arg_cache,
            Address::from_hex(CLOCK_OBJECT_ID).unwrap(),
        )
        .await
        .unwrap();
        let pool_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            &mut self.arg_cache,
            Address::from_hex(&pool_id).unwrap(),
        )
        .await
        .unwrap();
        let strategy_id_arg = argument::pure(&mut self.tx, request.strategy_id.clone()).unwrap();
        let current_tick_arg = argument::pure(&mut self.tx, request.current_tick_u32).unwrap();
        let current_sqrt_price_arg =
            argument::pure(&mut self.tx, request.current_sqrt_price).unwrap();
        let tick_spacing_arg = argument::pure(&mut self.tx, request.tick_spacing).unwrap();
        let tick_lower_index_arg =
            argument::pure(&mut self.tx, request.tick_lower_index_u32).unwrap();
        let tick_upper_index_arg =
            argument::pure(&mut self.tx, request.tick_upper_index_u32).unwrap();

        let construct_req = self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("auto_rebalance").unwrap(),
                Identifier::new("new_auto_rebalance_request").unwrap(),
                vec![],
            ),
            vec![
                strategy_id_arg,
                current_tick_arg,
                current_sqrt_price_arg,
                tick_spacing_arg,
                tick_lower_index_arg,
                tick_upper_index_arg,
            ],
        );

        let prepare_rebalance_data = self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("auto_rebalance").unwrap(),
                Identifier::new("prepare_rebalance_bot").unwrap(),
                vec![TypeTag::Struct(Box::new(
                    StructTag::from_str(pos_type).unwrap(),
                ))],
            ),
            vec![
                strategy_arg,
                global_config_arg,
                registry_arg,
                enclave_arg,
                timestamp_arg,
                construct_req,
                signature_arg,
                clock_arg,
            ],
        );

        let pos = prepare_rebalance_data.nested(0).unwrap();
        let receipt = prepare_rebalance_data.nested(1).unwrap();
        let coin_receipt = prepare_rebalance_data.nested(2).unwrap();

        let position = match dex {
            SupportedDex::Cetus => {
                let mut cetus_tx = cetus::CetusTransactionBuilder::new(
                    self.client,
                    &mut self.tx,
                    &mut self.arg_cache,
                )
                .await;
                cetus_tx
                    .rebalance(cetus::RebalanceData::new(
                        pos,
                        pool_arg,
                        coin_a_type.clone(),
                        coin_b_type.clone(),
                        helper::tick_to_i32(request.tick_lower_index_u32),
                        helper::tick_to_i32(request.tick_upper_index_u32),
                        request.current_sqrt_price,
                        liquidity,
                        helper::tick_to_i32(new_tick_lower_index),
                        helper::tick_to_i32(new_tick_upper_index),
                        lp_slippage_tolerance_bps,
                        coin_receipt,
                        rewarder_coin_types,
                    ))
                    .await
            }
            _ => panic!("Unsupported dex"),
        };

        let tick_lower_index_arg = argument::pure(&mut self.tx, new_tick_lower_index).unwrap();
        let tick_upper_index_arg = argument::pure(&mut self.tx, new_tick_upper_index).unwrap();
        let tick_lower_index = self.tx.move_call(
            Function::new(
                Address::from_hex(INTEGER_MATE_PACKAGE_ID).unwrap(),
                Identifier::new("i32").unwrap(),
                Identifier::new("from_u32").unwrap(),
                vec![],
            ),
            vec![tick_lower_index_arg],
        );
        let tick_upper_index = self.tx.move_call(
            Function::new(
                Address::from_hex(INTEGER_MATE_PACKAGE_ID).unwrap(),
                Identifier::new("i32").unwrap(),
                Identifier::new("from_u32").unwrap(),
                vec![],
            ),
            vec![tick_upper_index_arg],
        );
        let position_registry_id_arg = argument::pure(&mut self.tx, position_registry_id).unwrap();

        self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("registry").unwrap(),
                Identifier::new("return_position").unwrap(),
                vec![TypeTag::Struct(Box::new(
                    StructTag::from_str(pos_type).unwrap(),
                ))],
            ),
            vec![registry_arg, position, position_registry_id_arg],
        );

        self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("auto_rebalance").unwrap(),
                Identifier::new("repay_receipt").unwrap(),
                vec![TypeTag::Struct(Box::new(
                    StructTag::from_str(pos_type).unwrap(),
                ))],
            ),
            vec![
                registry_arg,
                strategy_arg,
                receipt,
                tick_lower_index,
                tick_upper_index,
                clock_arg,
            ],
        );

        self.tx
    }

    pub async fn compound(
        mut self,
        request: CompoundRequest,
        dex: SupportedDex,
        signature: Vec<u8>,
    ) -> TransactionBuilder {
        match dex {
            SupportedDex::Cetus => {
                let mut cetus_tx = cetus::CetusTransactionBuilder::new(
                    self.client,
                    &mut self.tx,
                    &mut self.arg_cache,
                )
                .await;
                cetus_tx.compound(request, signature).await;
            }
            _ => panic!("Unsupported dex"),
        }
        self.tx
    }
}
