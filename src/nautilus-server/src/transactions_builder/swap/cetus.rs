use std::ops::Add;

use sui_graphql_client::query_types::schema::__fields::SystemParameters::stakeSubsidyStartEpoch;
use sui_graphql_client::Client;
use sui_transaction_builder::{Function, TransactionBuilder};
use crate::transactions_builder::argument::{self, ArgCache};
use crate::transactions_builder::constant::{
    GLOBAL_CONFIG_ID,
    CETUS_INTEGRATE_PACKAGE_ID,
    REWARDERS_GLOBAL_VAULT_ID,
    CETUS_PACKAGE_ID,
    CETUS_PARTNER_ID,
    CETUS_AGGREGATOR_V1_PACKAGE_ID,
    CLOCK_OBJECT_ID,
};
use sui_sdk_types::{Address, Argument, Identifier, TypeTag};

pub struct CetusSwapAdapter<'a> {
    pub client: &'a Client,
    pub tx: &'a mut TransactionBuilder,
    pub arg_cache: &'a mut ArgCache,
}

impl<'a> CetusSwapAdapter<'a> {
    pub fn new(client: &'a Client, tx: &'a mut TransactionBuilder, arg_cache: &'a mut ArgCache) -> Self {
        Self { client, tx, arg_cache }
    }

    pub async fn swap_exact_in(
        &mut self,
        pool_id: String,
        from_type: TypeTag,
        to_type: TypeTag,
        direction: bool,
        coin_arg: Argument,
    ) -> Argument {
        let global_config_arg = argument::shared_mut_cached(self.client, self.tx, self.arg_cache, Address::from_hex(GLOBAL_CONFIG_ID).unwrap()).await.unwrap();
        let pool_arg = argument::shared_mut_cached(self.client, self.tx, self.arg_cache, Address::from_hex(pool_id).unwrap()).await.unwrap();
        let partner_arg = argument::shared_mut_cached(self.client, self.tx, self.arg_cache, Address::from_hex(CETUS_PARTNER_ID).unwrap()).await.unwrap();
        let clock_arg = argument::shared_ref_cached(self.client, self.tx, self.arg_cache, Address::from_hex(CLOCK_OBJECT_ID).unwrap()).await.unwrap();
        let func_name = if direction {
            "swap_a2b"
        } else {
            "swap_b2a"
        };
        let type_args = if direction {
            vec![from_type, to_type]
        } else {
            vec![to_type, from_type]
        };
        self.tx.move_call(
            Function::new(
                Address::from_hex(CETUS_AGGREGATOR_V1_PACKAGE_ID).unwrap(),
                Identifier::new("cetus").unwrap(),
                Identifier::new(func_name).unwrap(),
                type_args,
            ),
            vec![
                global_config_arg,
                pool_arg,
                partner_arg,
                coin_arg,
                clock_arg,
            ],
        )
    }
}