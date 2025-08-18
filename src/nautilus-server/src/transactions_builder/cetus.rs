use sui_sdk_types::Address;
use sui_sdk_types::Identifier;
use sui_sdk_types::Transaction;
use sui_sdk_types::TypeTag;
use sui_transaction_builder::{Function, TransactionBuilder};

use crate::parsers::CompoundRequest;
use crate::parsers::RebalanceRequest;
use crate::parsers::Request;
use crate::transactions_builder::helper;
use sui_graphql_client::Client;
use sui_sdk_types::Argument;
use crate::transactions_builder::argument;

pub struct ZapOutRequest {
    pub position: Argument,
    pub pool_arg: Argument,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
}

impl ZapOutRequest {
    pub fn new(position: Argument, pool_arg: Argument, coin_a_type: TypeTag, coin_b_type: TypeTag) -> Self {
        Self { position, pool_arg, coin_a_type, coin_b_type }
    }
}

pub struct ZapInRequest {
    pub coin_a: Argument,
    pub coin_b: Argument,
    pub pool_arg: Argument,
    pub tick_lower_index: u32,
    pub tick_upper_index: u32,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
}

impl ZapInRequest {
    pub fn new(coin_a: Argument, coin_b: Argument, pool_arg: Argument, coin_a_type: TypeTag, coin_b_type: TypeTag, tick_lower_index: u32, tick_upper_index: u32) -> Self {
        Self { coin_a, coin_b, pool_arg, coin_a_type, coin_b_type, tick_lower_index, tick_upper_index }
    }
}

pub struct RebalanceData {
    pub position: Argument,
    pub pool_arg: Argument,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
    pub tick_lower_index: u32,
    pub tick_upper_index: u32,
    pub clock_arg: Argument,
}

impl RebalanceData {
    pub fn new(position: Argument, pool_arg: Argument, coin_a_type: TypeTag, coin_b_type: TypeTag, tick_lower_index: u32, tick_upper_index: u32, clock_arg: Argument) -> Self {
        Self { position, pool_arg, coin_a_type, coin_b_type, tick_lower_index, tick_upper_index, clock_arg }
    }
}


const CETUS_INTEGRATE_PACKAGE_ID: &str = "0xb2db7142fa83210a7d78d9c12ac49c043b3cbbd482224fea6e3da00aa5a5ae2d";
const CETUS_PACKAGE_ID: &str = "0x75b2e9ecad34944b8d0c874e568c90db0cf9437f0d7392abfd4cb902972f3e40";
const GLOBAL_CONFIG_PACKAGE_ID: &str = "0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f";

pub struct CetusTransactionBuilder<'a> {
    pub client: &'a Client,
    pub tx: &'a mut TransactionBuilder,
    global_config_arg: Argument,
}

impl<'a> CetusTransactionBuilder<'a> {
    pub async fn new(client: &'a Client, tx: &'a mut TransactionBuilder) -> Self {
        let global_config_arg = argument::shared_ref(client, tx, Address::from_hex(GLOBAL_CONFIG_PACKAGE_ID).unwrap()).await.unwrap();
        Self { client, tx, global_config_arg }
    }

    pub async fn zap_out(&mut self, request: ZapOutRequest, clock_arg: Argument) -> (Argument, Argument) {
        let true_arg = argument::pure(&mut self.tx, true).unwrap();

        let result = self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_INTEGRATE_PACKAGE_ID).unwrap(),
                Identifier::new("pool_script_v2").unwrap(),
                Identifier::new("close_position_with_return").unwrap(),
                vec![request.coin_a_type, request.coin_b_type],
            ),
            vec![self.global_config_arg, request.pool_arg, request.position, true_arg, clock_arg],
        );
        (result.nested(0).unwrap(), result.nested(1).unwrap())
    }

    pub async fn zap_in(&mut self, request: ZapInRequest, clock_arg: Argument) -> (Argument) {
        let tick_lower_index_arg = argument::pure(&mut self.tx, request.tick_lower_index).unwrap();
        let tick_upper_index_arg = argument::pure(&mut self.tx, request.tick_upper_index).unwrap();
        let true_arg = argument::pure(&mut self.tx, true).unwrap();
        let max_amount_arg = argument::pure(&mut self.tx, u64::MAX).unwrap();
        
        let position = self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_PACKAGE_ID).unwrap(),
                Identifier::new("pool").unwrap(),
                Identifier::new("open_position").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![self.global_config_arg, request.pool_arg, tick_lower_index_arg, tick_upper_index_arg],
        );

        self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_INTEGRATE_PACKAGE_ID).unwrap(),
                Identifier::new("pool_script_v2").unwrap(),
                Identifier::new("add_liquidity_by_fix_coin").unwrap(),
                vec![request.coin_a_type, request.coin_b_type],
            ),
            vec![self.global_config_arg, request.pool_arg, position, request.coin_a, request.coin_b, max_amount_arg, max_amount_arg, true_arg, clock_arg],
        );
        (position)
    }

    pub async fn rebalance(&mut self, rebalance_data: RebalanceData) -> (Argument) {
        let zap_out_request = ZapOutRequest::new(rebalance_data.position, rebalance_data.pool_arg.clone(), rebalance_data.coin_a_type.clone(), rebalance_data.coin_b_type.clone());
        let (coin_a, coin_b) = self.zap_out(zap_out_request, rebalance_data.clock_arg).await;
        let zap_in_request = ZapInRequest::new(coin_a, coin_b, rebalance_data.pool_arg.clone(), rebalance_data.coin_a_type.clone(), rebalance_data.coin_b_type.clone(), rebalance_data.tick_lower_index, rebalance_data.tick_upper_index);
        let position = self.zap_in(zap_in_request, rebalance_data.clock_arg).await;
        position
    }

    pub async fn compound(&mut self, request: CompoundRequest, signature: Vec<u8>) {
        // let tx = self.zap_out(tx);
        // let tx = self.zap_in(tx);
        // tx
    }
}
