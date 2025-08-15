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

pub struct CetusTransactionBuilder {
}

impl CetusTransactionBuilder {
    pub fn new() -> Self {
        Self {  }
    }

    pub fn zap_out(&self, tx: TransactionBuilder) -> TransactionBuilder {
        // transaction_builder.move_call(
        //     Function::new(
        //         Address::from_hex(&self.kurage_package_id).unwrap(),
        //         Identifier::new("auto_rebalance").unwrap(),
        //         Identifier::new("prepare_rebalance_bot").unwrap(),
        //         self.type_args,
        //     ),
        //     vec![],
        // );
        // transaction_builder.finish().unwrap()
        tx
    }

    pub fn zap_in(&self, tx: TransactionBuilder) -> TransactionBuilder {
        tx
    }

    pub async fn rebalance(self, tx: TransactionBuilder, pos: Argument, receipt: Argument) -> (TransactionBuilder, Argument, Argument) {
        let tx = self.zap_out(tx);
        let tx = self.zap_in(tx);
        (tx, pos, receipt)
    }

    pub async fn compound(self, request: CompoundRequest, signature: Vec<u8>, tx: TransactionBuilder) -> TransactionBuilder {
        let tx = self.zap_out(tx);
        let tx = self.zap_in(tx);
        tx
    }
}
