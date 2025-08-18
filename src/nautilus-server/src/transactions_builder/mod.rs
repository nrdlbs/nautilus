use crate::parsers::{CompoundRequest, RebalanceRequest, SupportedDex};
use sui_graphql_client::query_types::schema::__fields::SystemParameters::stakeSubsidyStartEpoch;
use sui_graphql_client::Client;
use sui_sdk_types::{Address, Identifier, Input, ObjectReference, StructTag, TypeTag};
use sui_transaction_builder::{Function, TransactionBuilder};
use std::collections::HashMap;
use std::str::FromStr;
pub mod cetus;
pub mod helper;
pub mod argument;

const KURAGE_PACKAGE_ID: &str = "0x346acd3d4e93b389463c19cb17f698c0a5704abc3a26c9e888f796e240b47250";
const KURAGE_NEWEST_PACKAGE_ID: &str = "0xaf632fb3ed93dbf0def0f09cc1291b0e89c61e476b8e088a7c09fcf841f2ce79";
const INTEGER_MATE_PACKAGE_ID: &str = "0x714a63a0dba6da4f017b42d5d0fb78867f18bcde904868e51d951a5a6f5b7f57";
const REGISTRY_OBJECT_ID: &str = "0x0a02d97ed7624a1952d3a231b4d6b76de1308c20237654b2eb063aaf1b961785";
const CLOCK_OBJECT_ID: &str = "0x0000000000000000000000000000000000000000000000000000000000000006";

// array of position types
fn get_position_types(dex: SupportedDex) -> &'static str {
    match dex {
        SupportedDex::Cetus => "0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb::position::Position",
        _ => panic!("Unsupported dex"),
    }
}

pub struct DexTransactionBuilder<'a> {
    tx: TransactionBuilder,
    client: &'a Client,
}

impl<'a> DexTransactionBuilder<'a> {
    pub async fn new(client: &'a Client, caller: Address, gas_budget: u64) -> Self {
        let tx = helper::new_with_gas(client, caller, gas_budget)
            .await
            .unwrap();
        Self { tx, client }
    }

    pub async fn rebalance(
        mut self,
        request: RebalanceRequest,
        pool_id: String,
        coin_a_type: TypeTag,
        coin_b_type: TypeTag,
        new_tick_lower_index: u32,
        new_tick_upper_index: u32,
        position_registry_id: u64,
        dex: SupportedDex,
        enclave_id: String,
        signature: Vec<u8>,
        timestamp_ms: u64,
    ) -> TransactionBuilder {
        let pos_type = get_position_types(dex);
        
        let registry_arg = argument::shared_mut(self.client, &mut self.tx, Address::from_hex(REGISTRY_OBJECT_ID).unwrap()).await.unwrap();
        let strategy_arg = argument::shared_mut(self.client, &mut self.tx, request.strategy_id.clone()).await.unwrap();
        let enclave_arg = argument::shared_mut(self.client, &mut self.tx, Address::from_hex(&enclave_id).unwrap()).await.unwrap();
        let timestamp_arg = argument::pure(&mut self.tx, timestamp_ms).unwrap();
        let signature_arg = argument::pure(&mut self.tx, signature).unwrap();
        let clock_arg = argument::shared_ref(self.client, &mut self.tx, Address::from_hex(CLOCK_OBJECT_ID).unwrap()).await.unwrap();
        let pool_arg = argument::shared_mut(self.client, &mut self.tx, Address::from_hex(&pool_id).unwrap()).await.unwrap();
        let strategy_id_arg = argument::pure(&mut self.tx, Address::from_hex(&request.strategy_id).unwrap()).unwrap();
        let current_tick_arg = argument::pure(&mut self.tx, request.current_tick_u32).unwrap();
        let current_sqrt_price_arg = argument::pure(&mut self.tx, request.current_sqrt_price).unwrap();
        let tick_spacing_arg = argument::pure(&mut self.tx, request.tick_spacing).unwrap();
        let tick_lower_index_arg = argument::pure(&mut self.tx, request.tick_lower_index_u32).unwrap();
        let tick_upper_index_arg = argument::pure(&mut self.tx, request.tick_upper_index_u32).unwrap();

        let construct_req = self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_NEWEST_PACKAGE_ID).unwrap(),
                Identifier::new("auto_rebalance").unwrap(),
                Identifier::new("new_auto_rebalance_request").unwrap(),
                vec![],
            ),
            vec![strategy_id_arg, current_tick_arg, current_sqrt_price_arg, tick_spacing_arg, tick_lower_index_arg, tick_upper_index_arg],
        );

        let prepare_rebalance_data = self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_NEWEST_PACKAGE_ID).unwrap(),
                Identifier::new("auto_rebalance").unwrap(),
                Identifier::new("prepare_rebalance_bot").unwrap(),
                vec![TypeTag::Struct(Box::new(StructTag::from_str(pos_type).unwrap()))],
            ),
            vec![registry_arg, strategy_arg, enclave_arg, timestamp_arg, construct_req, signature_arg, clock_arg],
        );

        let pos = prepare_rebalance_data.nested(0).unwrap();
        let receipt = prepare_rebalance_data.nested(1).unwrap();

        let position = match dex {
            SupportedDex::Cetus => {
                let mut cetus_tx = cetus::CetusTransactionBuilder::new(self.client, &mut self.tx).await;
                cetus_tx.rebalance(cetus::RebalanceData::new(pos, pool_arg, coin_a_type.clone(), coin_b_type.clone(), request.tick_lower_index_u32, request.tick_upper_index_u32, clock_arg)).await
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
                Address::from_hex(KURAGE_NEWEST_PACKAGE_ID).unwrap(),
                Identifier::new("registry").unwrap(),
                Identifier::new("return_position").unwrap(),
                vec![TypeTag::Struct(Box::new(StructTag::from_str(pos_type).unwrap()))],
            ),
            vec![
                registry_arg,
                position,
                position_registry_id_arg,
            ],
        );

        self.tx.move_call(
            Function::new(
                Address::from_hex(KURAGE_NEWEST_PACKAGE_ID).unwrap(),
                Identifier::new("auto_rebalance").unwrap(),
                Identifier::new("repay_receipt").unwrap(),
                vec![TypeTag::Struct(Box::new(StructTag::from_str(pos_type).unwrap()))],
            ),
            vec![
                registry_arg,
                strategy_arg,
                receipt,
                tick_lower_index,
                tick_upper_index,
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
                let mut cetus_tx = cetus::CetusTransactionBuilder::new(self.client, &mut self.tx).await;
                cetus_tx.compound(request, signature).await;
            }
            _ => panic!("Unsupported dex"),
        }
        self.tx
    }
}
