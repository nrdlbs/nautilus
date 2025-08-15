// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]

use crate::common::parse_sui_privkey;
use crate::common::IntentMessage;
use crate::common::{
    construct_kp_from_bech32_string, to_signed_response, IntentScope, ProcessDataRequest,
    ProcessedDataResponse,
};
use crate::parsers;
use crate::transactions_builder::helper;
use crate::transactions_builder::DexTransactionBuilder;
use crate::AppState;
use crate::EnclaveError;
use anyhow::anyhow;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sui_crypto::simple::SimpleKeypair;
use sui_crypto::Signer;
use sui_crypto::SuiSigner;
use sui_graphql_client::Client as GraphQLClient;
use sui_graphql_client::PaginationFilter;
use sui_rpc::field::FieldMask;
use sui_rpc::proto::sui::rpc::v2beta2::GetBalanceRequest;
use sui_rpc::proto::sui::rpc::v2beta2::GetObjectRequest;
use sui_rpc::Client;
use sui_sdk_types::Input;
use sui_sdk_types::Object;
use sui_sdk_types::TransactionEffects;
use sui_sdk_types::{Ed25519PublicKey, MultisigMemberPublicKey};

/// ====
/// Core Nautilus server logic, replace it with your own
/// relavant structs and process_data endpoint.
/// ====
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionRequest {
    pub pool_id: String,
    pub strategy_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionResponse {
    pub request: parsers::Request,
}

pub async fn process_data_v2(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<TransactionRequest>>,
) -> Result<(), EnclaveError> {
    let kp = construct_kp_from_bech32_string(&state.pk_string)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to construct keypair: {}", e)))?;
    let mut client = Client::new("https://fullnode.mainnet.sui.io:443")
        .map_err(|e| EnclaveError::GenericError(format!("Failed to create client: {}", e)))?;
    let mut ledger_client = client.ledger_client();
    let mut graphql_client = GraphQLClient::new_mainnet();

    let pool_data = ledger_client
        .get_object(GetObjectRequest {
            object_id: Some(request.payload.pool_id.clone()),
            version: None,
            read_mask: Some(FieldMask {
                paths: vec!["*".into()],
            }),
        })
        .await
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get object: {}", e)))?;
    let pool_object = pool_data.into_inner().object.unwrap();

    let strategy_data = ledger_client
        .get_object(GetObjectRequest {
            object_id: Some(request.payload.strategy_id.clone()),
            version: None,
            read_mask: Some(FieldMask {
                paths: vec!["*".into()],
            }),
        })
        .await
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get object: {}", e)))?;
    let strategy_object = strategy_data.into_inner().object.unwrap();

    let (request, dex) = parsers::into_request(&mut graphql_client, pool_object, strategy_object)
        .await
        .map_err(|e| EnclaveError::GenericError(format!("Failed to handle object: {}", e)))?;
    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| EnclaveError::GenericError(format!("Failed to get current timestamp: {}", e)))?
        .as_millis() as u64;

    let address = kp.public_key().derive_address();
    let dex_tx_builder = DexTransactionBuilder::new(&graphql_client, address, 100000000).await;

    let signed_data = to_signed_response(
        &state.eph_kp,
        TransactionResponse {
            request: request.clone(),
        },
        current_timestamp,
        IntentScope::Transaction,
    );

    let tx = match &request {
        parsers::Request::Rebalance(rebalance_req) => dex_tx_builder.rebalance(
            rebalance_req.clone(),
            rebalance_req.tick_lower_index_u32,
            rebalance_req.tick_upper_index_u32,
            dex,
            signed_data.signature.clone().into_bytes(),
            current_timestamp,
        ).await,
        parsers::Request::Compound(compound_req) => dex_tx_builder.compound(
            compound_req.clone(),
            dex,
            signed_data.signature.clone().into_bytes(),
        ).await,
    };

    match helper::execute_and_wait_for_effects(&graphql_client, tx, &kp).await {
        Ok(effects) => {
            let transaction_digest = match effects {
                TransactionEffects::V1(effects) => {
                    effects.transaction_digest
                }
                TransactionEffects::V2(effects) => {
                    effects.transaction_digest
                }
            };
            println!("transaction_digest: {:?}", transaction_digest);
        }
        Err(e) => {
            println!("Error executing transaction: {:?}", e);
        }
    }

    Ok(())
}
