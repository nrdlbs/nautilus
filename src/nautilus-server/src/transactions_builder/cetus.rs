use num_bigint::BigInt;
use sui_graphql_client::query_types::schema::__fields::SplitCoinsTransaction::coin;
use sui_sdk_types::Address;
use sui_sdk_types::Identifier;
use sui_sdk_types::StructTag;
use sui_sdk_types::Transaction;
use sui_sdk_types::TypeTag;
use sui_transaction_builder::{Function, TransactionBuilder};

use crate::aggregator;
use crate::math::clmm_math;
use crate::math::tick_math;
use crate::parsers::CompoundRequest;
use crate::parsers::RebalanceRequest;
use crate::parsers::Request;
use crate::parsers::SupportedDex;
use crate::transactions_builder::argument;
use crate::transactions_builder::argument::ArgCache;
use crate::transactions_builder::constant::CETUS_AGGREGATOR_V1_PACKAGE_ID;
use crate::transactions_builder::constant::KURAGE_PACKAGE_ID;
use crate::transactions_builder::constant::REGISTRY_OBJECT_ID;
use crate::transactions_builder::constant::{
    CETUS_INTEGRATE_PACKAGE_ID, CETUS_PACKAGE_ID, GLOBAL_CONFIG_ID, REWARDERS_GLOBAL_VAULT_ID,
};
use crate::transactions_builder::get_position_types;
use crate::transactions_builder::helper;
use crate::transactions_builder::swap::cetus::CetusSwapAdapter;
use crate::transactions_builder::CLOCK_OBJECT_ID;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use sui_graphql_client::Client;
use sui_sdk_types::Argument;
use num_traits::ToPrimitive;

#[derive(Clone)]
pub struct ZapOutRequest {
    pub position: Argument,
    pub pool_arg: Argument,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
}

impl ZapOutRequest {
    pub fn new(
        position: Argument,
        pool_arg: Argument,
        coin_a_type: TypeTag,
        coin_b_type: TypeTag,
        sqrt_price_x64: u128,
        liquidity: u128,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Self {
        Self {
            position,
            pool_arg,
            coin_a_type,
            coin_b_type,
            sqrt_price_x64,
            liquidity,
            tick_lower_index,
            tick_upper_index,
        }
    }
}

pub struct ZapInRequest {
    pub coin_a: Argument,
    pub pool_arg: Argument,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
    pub sqrt_price_x64: u128,
    pub coin_a_amount: u64,
    pub slippage_tolerance: u64,

}

impl ZapInRequest {
    pub fn new(
        coin_a: Argument,
        pool_arg: Argument,
        coin_a_type: TypeTag,
        coin_b_type: TypeTag,
        tick_lower_index: i32,
        tick_upper_index: i32,
        sqrt_price_x64: u128,
        coin_a_amount: u64,
        slippage_tolerance: u64,
    ) -> Self {
        Self {
            coin_a,
            pool_arg,
            coin_a_type,
            coin_b_type,
            tick_lower_index,
            tick_upper_index,
            sqrt_price_x64,
            coin_a_amount,
            slippage_tolerance,
        }
    }
}

pub struct RebalanceData {
    pub position: Argument,
    pub pool_arg: Argument,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
    pub current_tick_lower_index: i32,
    pub current_tick_upper_index: i32,
    pub current_sqrt_price: u128,
    pub current_position_liquidity: u128,
    pub new_tick_lower_index: i32,
    pub new_tick_upper_index: i32,
    pub lp_slippage_tolerance_bps: u64,
    pub coin_receipt: Argument,
    pub rewarder_coin_types: Vec<TypeTag>,
}

impl RebalanceData {
    pub fn new(
        position: Argument,
        pool_arg: Argument,
        coin_a_type: TypeTag,
        coin_b_type: TypeTag,
        current_tick_lower_index: i32,
        current_tick_upper_index: i32,
        current_sqrt_price: u128,
        current_position_liquidity: u128,
        new_tick_lower_index: i32,
        new_tick_upper_index: i32,
        lp_slippage_tolerance_bps: u64,
        coin_receipt: Argument,
        rewarder_coin_types: Vec<TypeTag>,
    ) -> Self {
        Self {
            position,
            pool_arg,
            coin_a_type,
            coin_b_type,
            current_tick_lower_index,
            current_tick_upper_index,
            current_sqrt_price,
            current_position_liquidity,
            new_tick_lower_index,
            new_tick_upper_index,
            lp_slippage_tolerance_bps,
            coin_receipt,
            rewarder_coin_types,
        }
    }
}

pub struct CollectFeesAndRewardsRequest {
    pub position: Argument,
    pub pool_arg: Argument,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
    pub rewarder_coin_types: Vec<TypeTag>,
    pub is_collect_fees: bool,
    pub is_collect_rewards: bool,
}

impl CollectFeesAndRewardsRequest {
    pub fn new(
        position: Argument,
        pool_arg: Argument,
        coin_a_type: TypeTag,
        coin_b_type: TypeTag,
        rewarder_coin_types: Vec<TypeTag>,
        is_collect_fees: bool,
        is_collect_rewards: bool,
    ) -> Self {
        Self {
            position,
            pool_arg,
            coin_a_type,
            coin_b_type,
            rewarder_coin_types,
            is_collect_fees,
            is_collect_rewards,
        }
    }
}

pub struct CollectFeesAndRewardsResult {
    pub coin_type: TypeTag,
    pub coin: Argument,
}

pub struct SwapByAmountInRequest {
    pub from: TypeTag,
    pub to: TypeTag,
    pub amount_in: u64,
    pub coin_input: Argument,
    pub route: Option<aggregator::cetus::RouteData>,
}

impl SwapByAmountInRequest {
    pub fn new(from: TypeTag, to: TypeTag, amount_in: u64, coin_input: Argument, route: Option<aggregator::cetus::RouteData>) -> Self {
        Self {
            from,
            to,
            amount_in,
            coin_input,
            route,
        }
    }
}
pub struct CetusTransactionBuilder<'a> {
    pub client: &'a Client,
    pub tx: &'a mut TransactionBuilder,
    pub arg_cache: &'a mut ArgCache,
}

impl<'a> CetusTransactionBuilder<'a> {
    pub async fn new(
        client: &'a Client,
        tx: &'a mut TransactionBuilder,
        arg_cache: &'a mut ArgCache,
    ) -> Self {
        Self {
            client,
            tx,
            arg_cache,
        }
    }

    pub async fn collect_fees_and_rewards(
        &mut self,
        request: CollectFeesAndRewardsRequest,
    ) -> Vec<CollectFeesAndRewardsResult> {
        let mut result = vec![];
        if request.is_collect_fees {
            let new_coin_a =
                argument::zero_coin(&mut self.tx, request.coin_a_type.clone()).unwrap();
            let new_coin_b =
                argument::zero_coin(&mut self.tx, request.coin_b_type.clone()).unwrap();
            let global_config_arg = argument::shared_mut_cached(
                self.client,
                &mut self.tx,
                self.arg_cache,
                Address::from_hex(GLOBAL_CONFIG_ID).unwrap(),
            )
            .await
            .unwrap();
            self.tx.move_call(
                Function::new(
                    Address::from_hex(CETUS_INTEGRATE_PACKAGE_ID).unwrap(),
                    Identifier::new("pool_script_v3").unwrap(),
                    Identifier::new("collect_fee").unwrap(),
                    vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
                ),
                vec![
                    global_config_arg,
                    request.pool_arg,
                    request.position,
                    new_coin_a,
                    new_coin_b,
                ],
            );
            result.push(CollectFeesAndRewardsResult {
                coin_type: request.coin_a_type.clone(),
                coin: new_coin_a,
            });
            result.push(CollectFeesAndRewardsResult {
                coin_type: request.coin_b_type.clone(),
                coin: new_coin_b,
            });
        }
        if request.is_collect_rewards {
            let global_vault_arg = argument::shared_mut_cached(
                self.client,
                &mut self.tx,
                self.arg_cache,
                Address::from_hex(REWARDERS_GLOBAL_VAULT_ID).unwrap(),
            )
            .await
            .unwrap();
            let global_config_arg = argument::shared_mut_cached(
                self.client,
                &mut self.tx,
                self.arg_cache,
                Address::from_hex(GLOBAL_CONFIG_ID).unwrap(),
            )
            .await
            .unwrap();
            for rewarder_coin_type in &request.rewarder_coin_types {
                let new_rewarder_coin =
                    argument::zero_coin(&mut self.tx, rewarder_coin_type.clone()).unwrap();
                let clock_arg = argument::shared_ref_cached(
                    self.client,
                    &mut self.tx,
                    self.arg_cache,
                    Address::from_hex(CLOCK_OBJECT_ID).unwrap(),
                )
                .await
                .unwrap();
                self.tx.move_call(
                    Function::new(
                        Address::from_hex(CETUS_INTEGRATE_PACKAGE_ID).unwrap(),
                        Identifier::new("pool_script_v3").unwrap(),
                        Identifier::new("collect_reward").unwrap(),
                        vec![
                            request.coin_a_type.clone(),
                            request.coin_b_type.clone(),
                            rewarder_coin_type.clone(),
                        ],
                    ),
                    vec![
                        global_config_arg,
                        request.pool_arg,
                        request.position,
                        global_vault_arg,
                        new_rewarder_coin,
                        clock_arg,
                    ],
                );
                result.push(CollectFeesAndRewardsResult {
                    coin_type: rewarder_coin_type.clone(),
                    coin: new_rewarder_coin,
                });
            }
        }
        result
    }

    pub async fn swap_by_amount_in(&mut self, swap_request: SwapByAmountInRequest) -> (Argument, u64) {
        let cetus_aggregator = aggregator::cetus::CetusAggregator::new();
        let route = if swap_request.route.is_some() {
            swap_request.route.unwrap().clone()
        } else {
            let route = cetus_aggregator.swap_by_amount_in(aggregator::cetus::SwapByAmountInRequest {
                from: swap_request.from.to_string(),
                to: swap_request.to.to_string(),
                amount_in: swap_request.amount_in,
            }).await.unwrap();
            route.data.unwrap()
        };
        let coin_input = swap_request.coin_input;
        let new_coin = argument::zero_coin(&mut self.tx, swap_request.to).unwrap();
        for i in 0..route.routes.len() {
            let swap_route = &route.routes[i];            
            let mut coin_in_this_route = if i == route.routes.len() - 1 {
                coin_input
            } else {
                let swap_amount_in_arg = argument::pure(self.tx, swap_route.amount_in).unwrap();
                self.tx.split_coins(coin_input, vec![swap_amount_in_arg])
            };
            let mut cetus_swap_adapter = CetusSwapAdapter::new(self.client, self.tx, self.arg_cache);
            for j in 0..swap_route.path.len() {
                let pool_id = swap_route.path[j].id.clone();
                let from_type = TypeTag::from_str(&swap_route.path[j].from).unwrap();
                let to_type = TypeTag::from_str(&swap_route.path[j].target).unwrap();
                let coin_output = cetus_swap_adapter
                    .swap_exact_in(               
                        pool_id,
                        from_type,
                        to_type,
                        swap_route.path[j].direction,
                        coin_in_this_route,
                    )
                    .await;
                coin_in_this_route = coin_output;
            }
            self.tx.merge_coins(new_coin, vec![coin_in_this_route]);
        }
        (new_coin, route.routes.iter().map(|route| route.amount_out).sum::<u64>())
    }

    pub async fn zap_out(
        &mut self,
        request: ZapOutRequest,
    ) -> (Argument, u64) {
        let true_arg = argument::pure(&mut self.tx, true).unwrap();
        let global_config_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            self.arg_cache,
            Address::from_hex(GLOBAL_CONFIG_ID).unwrap(),
        )
        .await
        .unwrap();
        let clock_arg = argument::shared_ref_cached(
            self.client,
            &mut self.tx,
            self.arg_cache,
            Address::from_hex(CLOCK_OBJECT_ID).unwrap(),
        )
        .await
        .unwrap();
        let result = self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_INTEGRATE_PACKAGE_ID).unwrap(),
                Identifier::new("pool_script_v2").unwrap(),
                Identifier::new("close_position_with_return").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![
                global_config_arg,
                request.pool_arg,
                request.position,
                true_arg,
                clock_arg,
            ],
        );
        let coin_a = result.nested(0).unwrap();
        let coin_b = result.nested(1).unwrap();
        let (amount_a, amount_b) = clmm_math::calculate_amounts_by_liquidity(clmm_math::CalculateAmountsByLiquidityRequest {
            tick_lower_index: request.tick_lower_index,
            tick_upper_index: request.tick_upper_index,
            sqrt_price_x64: request.sqrt_price_x64,
            liquidity: request.liquidity,
        }).unwrap();
        let (coin_a_swap, amount_a_swap) = self
        .swap_by_amount_in(SwapByAmountInRequest::new(
            request.coin_b_type.clone(),
            request.coin_a_type.clone(),
            amount_b,
            coin_b,
            None,
        ))
        .await;
        self.tx.merge_coins(coin_a, vec![coin_a_swap]);
        (coin_a, amount_a_swap + amount_a)
    }

    pub async fn zap_in(&mut self, request: ZapInRequest) -> (Argument, Argument, Argument) {
        println!("zap in request slippage tolerance: {:?}", request.slippage_tolerance);
        let (amount_liquidity, route_data) = clmm_math::calculate_add_liquidity_only_coin_a_liquidity(clmm_math::CalculateAddLiquidityOnlyCoinARequest {
            tick_lower_index: request.tick_lower_index,
            tick_upper_index: request.tick_upper_index,
            sqrt_price_x64: request.sqrt_price_x64,
            coin_a_amount: request.coin_a_amount,
            coin_a_type: request.coin_a_type.clone(),
            coin_b_type: request.coin_b_type.clone(),
            max_remain_rate: 2000000,
        }).await;
        let (coin_a_amount_added, coin_b_amount_added) = clmm_math::calculate_amounts_by_liquidity(clmm_math::CalculateAmountsByLiquidityRequest {
            tick_lower_index: request.tick_lower_index,
            tick_upper_index: request.tick_upper_index,
            sqrt_price_x64: request.sqrt_price_x64,
            liquidity: amount_liquidity,
        }).unwrap();
        let coin_a_swap_amount = argument::pure(&mut self.tx, request.coin_a_amount - coin_a_amount_added).unwrap();
        let coin_a_swap = self.tx.split_coins(request.coin_a, vec![coin_a_swap_amount]);
        let (coin_b_swap, amount_b_swap) = self.swap_by_amount_in(SwapByAmountInRequest::new(
            request.coin_a_type.clone(),
            request.coin_b_type.clone(),
            request.coin_a_amount - coin_a_amount_added,
            coin_a_swap,
            route_data,
        )).await;
        let coin_a_amount_limit = coin_a_amount_added - coin_a_amount_added * 100000 / 100000;
        let coin_b_amount_limit = coin_b_amount_added - coin_b_amount_added * 100000 / 100000;
        let coin_a_amount_limit_arg = argument::pure(&mut self.tx, coin_a_amount_limit).unwrap();
        let coin_b_amount_limit_arg = argument::pure(&mut self.tx, coin_b_amount_limit).unwrap();
        self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("utils").unwrap(),
                Identifier::new("assert_coin_value").unwrap(),
                vec![request.coin_a_type.clone()],
            ),
            vec![
                request.coin_a,
                coin_a_amount_limit_arg,
            ],
        );
        self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("utils").unwrap(),
                Identifier::new("assert_coin_value").unwrap(),
                vec![request.coin_a_type.clone()],
            ),
            vec![
                request.coin_a,
                coin_b_amount_limit_arg,
            ],
        );
        let clock_arg = argument::shared_ref_cached(
            self.client,
            &mut self.tx,
            self.arg_cache,
            Address::from_hex(CLOCK_OBJECT_ID).unwrap(),
        )
        .await
        .unwrap();
        let tick_lower_index_arg = argument::pure(&mut self.tx, request.tick_lower_index).unwrap();
        let tick_upper_index_arg = argument::pure(&mut self.tx, request.tick_upper_index).unwrap();
        let true_arg = argument::pure(&mut self.tx, true).unwrap();
        let amount_a_input = argument::pure(&mut self.tx, coin_a_amount_added).unwrap();
        let amount_b_input = argument::pure(&mut self.tx, u64::MAX).unwrap();
        let global_config_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            self.arg_cache,
            Address::from_hex(GLOBAL_CONFIG_ID).unwrap(),
        )
        .await
        .unwrap();
        let position = self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_PACKAGE_ID).unwrap(),
                Identifier::new("pool").unwrap(),
                Identifier::new("open_position").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![
                global_config_arg,
                request.pool_arg,
                tick_lower_index_arg,
                tick_upper_index_arg,
            ],
        );

        let add_liquidity_receipt = self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_PACKAGE_ID).unwrap(),
                Identifier::new("pool").unwrap(),
                Identifier::new("add_liquidity_fix_coin").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![
                global_config_arg,
                request.pool_arg,
                position,
                amount_a_input,
                true_arg,
                clock_arg,
            ],
        );

        let amounts = self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_PACKAGE_ID).unwrap(),
                Identifier::new("pool").unwrap(),
                Identifier::new("add_liquidity_pay_amount").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![
                add_liquidity_receipt,
            ],
        );

        self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_PACKAGE_ID).unwrap(),
                Identifier::new("pool").unwrap(),
                Identifier::new("add_liquidity_pay_amount").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![
                add_liquidity_receipt,
            ],
        );

        let coin_inpput_a = self.tx.move_call(
            Function::new(
                Address::from_hex("0x2").unwrap(),
                Identifier::new("coin").unwrap(),
                Identifier::new("split").unwrap(),
                vec![request.coin_a_type.clone()],
            ),
            vec![
                request.coin_a,
                amounts.nested(0).unwrap(),
            ],
        );

        let balance_a = self.tx.move_call(
            Function::new(
                Address::from_hex("0x2").unwrap(),
                Identifier::new("coin").unwrap(),
                Identifier::new("into_balance").unwrap(),
                vec![request.coin_a_type.clone()],
            ),
            vec![
                coin_inpput_a,
            ],
        );

        let coin_input_b = self.tx.move_call(
            Function::new(
                Address::from_hex("0x2").unwrap(),
                Identifier::new("coin").unwrap(),
                Identifier::new("split").unwrap(),
                vec![request.coin_b_type.clone()],
            ),
            vec![
                coin_b_swap,
                amounts.nested(1).unwrap(),
            ],
        );

        let balance_b = self.tx.move_call(
            Function::new(
                Address::from_hex("0x2").unwrap(),
                Identifier::new("coin").unwrap(),
                Identifier::new("into_balance").unwrap(),
                vec![request.coin_b_type.clone()],
            ),
            vec![
                coin_input_b,
            ],
        );

        self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_PACKAGE_ID).unwrap(),
                Identifier::new("pool").unwrap(),
                Identifier::new("repay_add_liquidity").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![
                global_config_arg,
                request.pool_arg,
                balance_a,
                balance_b,
                add_liquidity_receipt,
            ],
        );

        (position, request.coin_a, coin_b_swap)
    }

    pub async fn rebalance(&mut self, rebalance_data: RebalanceData) -> Argument {
        let position_type = get_position_types(SupportedDex::Cetus);
        let registry_arg = argument::shared_mut_cached(
            self.client,
            &mut self.tx,
            self.arg_cache,
            Address::from_hex(REGISTRY_OBJECT_ID).unwrap(),
        )
        .await
        .unwrap();
        let collect_fees_and_rewards_request = CollectFeesAndRewardsRequest::new(
            rebalance_data.position.clone(),
            rebalance_data.pool_arg.clone(),
            rebalance_data.coin_a_type.clone(),
            rebalance_data.coin_b_type.clone(),
            rebalance_data.rewarder_coin_types.clone(),
            true,
            true,
        );
        let results = self
            .collect_fees_and_rewards(collect_fees_and_rewards_request)
            .await;
        for result in results {
            self.tx.move_call(
                Function::new(
                    Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                    Identifier::new("registry").unwrap(),
                    Identifier::new("add_coin").unwrap(),
                    vec![
                        TypeTag::Struct(Box::new(StructTag::from_str(position_type).unwrap())),
                        result.coin_type,
                    ],
                ),
                vec![
                    registry_arg,
                    rebalance_data.coin_receipt,
                    result.coin,
                ],
            );
        }

        let zap_out_request = ZapOutRequest::new(
            rebalance_data.position,
            rebalance_data.pool_arg.clone(),
            rebalance_data.coin_a_type.clone(),
            rebalance_data.coin_b_type.clone(),
            rebalance_data.current_sqrt_price,
            rebalance_data.current_position_liquidity,
            rebalance_data.current_tick_lower_index,
            rebalance_data.current_tick_upper_index,
        );
        let (coin_a, amount_coin_a) = self
            .zap_out(zap_out_request)
            .await;

        let zap_in_request = ZapInRequest::new(
            coin_a,
            rebalance_data.pool_arg.clone(),
            rebalance_data.coin_a_type.clone(),
            rebalance_data.coin_b_type.clone(),
            rebalance_data.new_tick_lower_index,
            rebalance_data.new_tick_upper_index,
            rebalance_data.current_sqrt_price,
            amount_coin_a,
            rebalance_data.lp_slippage_tolerance_bps,
        );
        let (position, coin_a, coin_b) = self.zap_in(zap_in_request).await;
        self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("registry").unwrap(),
                Identifier::new("add_coin").unwrap(),
                vec![
                    TypeTag::Struct(Box::new(StructTag::from_str(position_type).unwrap())),
                    rebalance_data.coin_a_type.clone(),
                ],
            ),
            vec![
                registry_arg,
                rebalance_data.coin_receipt,
                coin_a,
            ],
        );
        self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_PACKAGE_ID).unwrap(),
                Identifier::new("registry").unwrap(),
                Identifier::new("add_coin").unwrap(),
                vec![
                    TypeTag::Struct(Box::new(StructTag::from_str(position_type).unwrap())),
                    rebalance_data.coin_b_type.clone(),
                ],
            ),
            vec![
                registry_arg,
                rebalance_data.coin_receipt,
                coin_b,
            ],
        );
        position
    }

    pub async fn compound(&mut self, request: CompoundRequest, signature: Vec<u8>) {
        // let tx = self.zap_out(tx);
        // let tx = self.zap_in(tx);
        // tx
    }
}
