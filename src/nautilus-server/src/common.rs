// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::AppState;
use crate::EnclaveError;
use axum::{extract::State, Json};
use fastcrypto::traits::Signer;
use fastcrypto::{encoding::Encoding, traits::ToFromBytes};
use fastcrypto::{encoding::Hex, traits::KeyPair as FcKeyPair};
use nsm_api::api::{Request as NsmRequest, Response as NsmResponse};
use nsm_api::driver;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use anyhow::{Context, Result};
use bech32::{Hrp, decode};
use sui_crypto::ed25519::Ed25519PrivateKey;
use fastcrypto::ed25519::Ed25519KeyPair;
/// ==== COMMON TYPES ====

/// Intent message wrapper struct containing the intent scope and timestamp.
/// This standardizes the serialized payload for signing.
#[derive(Debug, Serialize, Deserialize)]
pub struct IntentMessage<T: Serialize> {
    pub intent: IntentScope,
    pub timestamp_ms: u64,
    pub data: T,
}

/// Intent scope enum. Add new scope here if needed, each corresponds to a
/// scope for signing. Replace in with your own intent per message type being signed by the enclave.
#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum IntentScope {
    Transaction = 0,
}

impl<T: Serialize + Debug> IntentMessage<T> {
    pub fn new(data: T, timestamp_ms: u64, intent: IntentScope) -> Self {
        Self {
            data,
            timestamp_ms,
            intent,
        }
    }
}

/// Wrapper struct containing the response (the intent message) and signature.
#[derive(Serialize, Deserialize)]
pub struct ProcessedDataResponse<T> {
    pub response: T,
    pub signature: String,
}

/// Wrapper struct containing the request payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessDataRequest<T> {
    pub payload: T,
}

/// Sign the bcs bytes of the the payload with keypair.
pub fn to_signed_response<T: Serialize + Clone>(
    kp: &Ed25519KeyPair,
    payload: T,
    timestamp_ms: u64,
    intent: IntentScope,
) -> ProcessedDataResponse<IntentMessage<T>> {
    println!("intent: {:?}", &intent);
    println!("timestamp_ms: {:?}", timestamp_ms);
    println!("payload: {}", serde_json::to_string(&payload).unwrap());

    let intent_msg = IntentMessage {
        intent,
        timestamp_ms,
        data: payload.clone(),
    };

    let signing_payload = bcs::to_bytes(&intent_msg).expect("should not fail");
    println!("signing_payload: {:?}", Hex::encode(signing_payload.as_slice()));
    let sig = kp.sign(&signing_payload);
    ProcessedDataResponse {
        response: intent_msg,
        signature: Hex::encode(sig),
    }
}

/// ==== HEALTHCHECK, GET ATTESTASTION ENDPOINT IMPL ====

/// Response for get attestation.
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAttestationResponse {
    /// Attestation document serialized in Hex.
    pub attestation: String,
}

/// Endpoint that returns an attestation committed
/// to the enclave's public key.
pub async fn get_attestation(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GetAttestationResponse>, EnclaveError> {
    info!("get attestation called");

    let pk = state.eph_kp.public();
    let fd = driver::nsm_init();

    // Send attestation request to NSM driver with public key set.
    let request = NsmRequest::Attestation {
        user_data: None,
        nonce: None,
        public_key: Some(ByteBuf::from(pk.as_bytes().to_vec())),
    };

    let response = driver::nsm_process_request(fd, request);
    match response {
        NsmResponse::Attestation { document } => {
            driver::nsm_exit(fd);
            Ok(Json(GetAttestationResponse {
                attestation: Hex::encode(document),
            }))
        }
        _ => {
            driver::nsm_exit(fd);
            Err(EnclaveError::GenericError(
                "unexpected response".to_string(),
            ))
        }
    }
}

/// Health check response.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Hex encoded public key booted on enclave.
    pub pk: String,
    /// Status of endpoint connectivity checks
    pub endpoints_status: HashMap<String, bool>,
}

/// Endpoint that health checks the enclave connectivity to all
/// domains and returns the enclave's public key.
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthCheckResponse>, EnclaveError> {
    let pk = state.eph_kp.public();

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| EnclaveError::GenericError(format!("Failed to create HTTP client: {}", e)))?;

    // Load allowed endpoints from YAML file
    let endpoints_status = match std::fs::read_to_string("allowed_endpoints.yaml") {
        Ok(yaml_content) => {
            match serde_yaml::from_str::<serde_yaml::Value>(&yaml_content) {
                Ok(yaml_value) => {
                    let mut status_map = HashMap::new();

                    if let Some(endpoints) =
                        yaml_value.get("endpoints").and_then(|e| e.as_sequence())
                    {
                        for endpoint in endpoints {
                            if let Some(endpoint_str) = endpoint.as_str() {
                                // Check connectivity to each endpoint
                                let url = if endpoint_str.contains(".amazonaws.com") {
                                    format!("https://{}/ping", endpoint_str)
                                } else {
                                    format!("https://{}", endpoint_str)
                                };

                                let is_reachable = match client.get(&url).send().await {
                                    Ok(response) => {
                                        if endpoint_str.contains(".amazonaws.com") {
                                            // For AWS endpoints, check if response body contains "healthy"
                                            match response.text().await {
                                                Ok(body) => body.to_lowercase().contains("healthy"),
                                                Err(e) => {
                                                    info!(
                                                        "Failed to read response body from {}: {}",
                                                        endpoint_str, e
                                                    );
                                                    false
                                                }
                                            }
                                        } else {
                                            // For non-AWS endpoints, check for 200 status
                                            response.status().is_success()
                                        }
                                    }
                                    Err(e) => {
                                        info!("Failed to connect to {}: {}", endpoint_str, e);
                                        false
                                    }
                                };

                                status_map.insert(endpoint_str.to_string(), is_reachable);
                                info!(
                                    "Checked endpoint {}: reachable = {}",
                                    endpoint_str, is_reachable
                                );
                            }
                        }
                    }

                    status_map
                }
                Err(e) => {
                    info!("Failed to parse YAML: {}", e);
                    HashMap::new()
                }
            }
        }
        Err(e) => {
            info!("Failed to read allowed_endpoints.yaml: {}", e);
            HashMap::new()
        }
    };

    Ok(Json(HealthCheckResponse {
        pk: Hex::encode(pk.as_bytes()),
        endpoints_status,
    }))
}

fn five_to_eight_relaxed(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 5 / 8 + 1);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;

    for &v in data {
        acc = (acc << 5) | (v as u32);
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    out // ignore leftover <8 bits
}

pub fn parse_sui_privkey(bech: &str) -> Result<Ed25519PrivateKey> {
    let (hrp, payload) = decode(bech).context("bech32 decode failed")?;

    // HRP must be "suiprivkey"
    let expected = Hrp::parse_unchecked("suiprivkey");
    anyhow::ensure!(hrp == expected, "unexpected HRP: {}", hrp);

    // ADAPTIVE: 5-bit or already 8-bit?
    let maxv = payload.iter().copied().max().unwrap_or(0);
    let bytes = if maxv <= 31 {
        five_to_eight_relaxed(&payload)
    } else {
        payload // already 8-bit
    };

    anyhow::ensure!(bytes.len() == 33, "expected 33 bytes, got {}", bytes.len());
    anyhow::ensure!(bytes[0] == 0x00, "not an ed25519 key (scheme={:#04x})", bytes[0]);

    let sk: [u8; 32] = bytes[1..].try_into().unwrap();
    let key = Ed25519PrivateKey::new(sk);
    Ok(key)
}

pub fn construct_kp_from_bech32_string(bech: &str) -> Result<Ed25519PrivateKey> {
    let key = parse_sui_privkey(bech)?;
    Ok(key)
}