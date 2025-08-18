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
    pub pool_id: String,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
}

impl ZapOutRequest {
    pub fn new(position: Argument, pool_id: String, coin_a_type: TypeTag, coin_b_type: TypeTag) -> Self {
        Self { position, pool_id, coin_a_type, coin_b_type }
    }
}

pub struct ZapInRequest {
    pub coin_a: Argument,
    pub coin_b: Argument,
    pub pool_id: String,
    pub tick_lower_index: u32,
    pub tick_upper_index: u32,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
}

impl ZapInRequest {
    pub fn new(coin_a: Argument, coin_b: Argument, pool_id: String, coin_a_type: TypeTag, coin_b_type: TypeTag, tick_lower_index: u32, tick_upper_index: u32) -> Self {
        Self { coin_a, coin_b, pool_id, coin_a_type, coin_b_type, tick_lower_index, tick_upper_index }
    }
}

const CETUS_INTEGRATE_PACKAGE_ID: &str = "0xb2db7142fa83210a7d78d9c12ac49c043b3cbbd482224fea6e3da00aa5a5ae2d";
const CETUS_PACKAGE_ID: &str = "0x75b2e9ecad34944b8d0c874e568c90db0cf9437f0d7392abfd4cb902972f3e40";
const GLOBAL_CONFIG_PACKAGE_ID: &str = "0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f";
const CLOCK_OBJECT_ID: &str = "0x0000000000000000000000000000000000000000000000000000000000000006";

pub struct CetusTransactionBuilder<'a> {
    pub client: &'a Client,
}

impl<'a> CetusTransactionBuilder<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn zap_out(&self, mut tx: TransactionBuilder, request: ZapOutRequest) -> (TransactionBuilder, Argument, Argument) {
        let pool_arg = argument::shared_mut(&self.client, &mut tx, Address::from_hex(request.pool_id).unwrap()).await.unwrap();
        let position_arg = argument::pure(&mut tx, request.position).unwrap();
        let global_config_arg = argument::shared_ref(&self.client, &mut tx, Address::from_hex(GLOBAL_CONFIG_PACKAGE_ID).unwrap()).await.unwrap();
        let true_arg = argument::pure(&mut tx, true).unwrap();
        let clock_arg = argument::shared_ref(&self.client, &mut tx, Address::from_hex(CLOCK_OBJECT_ID).unwrap()).await.unwrap();

        let result = tx.move_call(
            Function::new(
                Address::from_hex(CETUS_INTEGRATE_PACKAGE_ID).unwrap(),
                Identifier::new("pool_script_v2").unwrap(),
                Identifier::new("close_position_with_return").unwrap(),
                vec![request.coin_a_type, request.coin_b_type],
            ),
            vec![global_config_arg, pool_arg, position_arg, true_arg, clock_arg],
        );
        (tx, result.nested(0).unwrap(), result.nested(1).unwrap())
    }

    pub async fn zap_in(&self, mut tx: TransactionBuilder, request: ZapInRequest) -> (TransactionBuilder, Argument) {
        let pool_arg = argument::shared_mut(&self.client, &mut tx, Address::from_hex(request.pool_id).unwrap()).await.unwrap();
        let coin_a_arg = argument::pure(&mut tx, request.coin_a).unwrap();
        let coin_b_arg = argument::pure(&mut tx, request.coin_b).unwrap();
        let tick_lower_index_arg = argument::pure(&mut tx, request.tick_lower_index).unwrap();
        let tick_upper_index_arg = argument::pure(&mut tx, request.tick_upper_index).unwrap();
        let global_config_arg = argument::shared_ref(&self.client, &mut tx, Address::from_hex(GLOBAL_CONFIG_PACKAGE_ID).unwrap()).await.unwrap();
        let true_arg = argument::pure(&mut tx, true).unwrap();
        let clock_arg = argument::shared_ref(&self.client, &mut tx, Address::from_hex(CLOCK_OBJECT_ID).unwrap()).await.unwrap();
        let max_amount_arg = argument::pure(&mut tx, u64::MAX).unwrap();
        
        let position = tx.move_call(
            Function::new(
                Address::from_hex(CETUS_PACKAGE_ID).unwrap(),
                Identifier::new("pool").unwrap(),
                Identifier::new("open_position").unwrap(),
                vec![request.coin_a_type.clone(), request.coin_b_type.clone()],
            ),
            vec![global_config_arg, pool_arg, tick_lower_index_arg, tick_upper_index_arg],
        );

        tx.move_call(
            Function::new(
                Address::from_hex(CETUS_INTEGRATE_PACKAGE_ID).unwrap(),
                Identifier::new("pool_script_v2").unwrap(),
                Identifier::new("add_liquidity_by_fix_coin").unwrap(),
                vec![request.coin_a_type, request.coin_b_type],
            ),
            vec![global_config_arg, pool_arg, position, coin_a_arg, coin_b_arg, max_amount_arg, max_amount_arg, true_arg, clock_arg],
        );
        (tx, position)
    }

    pub async fn rebalance(self, tx: TransactionBuilder, zap_in_request: ZapInRequest, zap_out_request: ZapOutRequest) -> (TransactionBuilder, Argument) {
        let (tx, coin_a, coin_b) = self.zap_out(tx, zap_out_request).await;
        let (tx, position) = self.zap_in(tx, zap_in_request).await;
        (tx, position)
    }

    pub async fn compound(self, request: CompoundRequest, signature: Vec<u8>, tx: TransactionBuilder) -> TransactionBuilder {
        // let tx = self.zap_out(tx);
        // let tx = self.zap_in(tx);
        tx
    }
}
