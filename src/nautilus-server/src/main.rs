// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use axum::{routing::get, routing::post, Router};
use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};
use nautilus_server::app::{process_data_v2};
use nautilus_server::common::{get_attestation, health_check};
use nautilus_server::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use once_cell::sync::Lazy;
use anyhow::Context;
use hex;
use fastcrypto::traits::ToFromBytes;

// ---------- static keypair ----------

#[tokio::main]
async fn main() -> Result<()> {
    // let eph_kp = Ed25519KeyPair::generate(&mut rand::thread_rng());
    // static eph_kp
    let seed_hex = std::env::var("EPH_ED25519_SEED")
        .context("EPH_ED25519_SEED not set (expect 32-byte hex seed)")?;
    let seed_hex = seed_hex.trim_start_matches("0x");
    let bytes = hex::decode(seed_hex).context("seed not valid hex")?;
    anyhow::ensure!(bytes.len() == 32, "seed must be exactly 32 bytes");

    // fastcrypto's Ed25519KeyPair can be deterministically derived from a 32-byte seed:
    let seed: [u8; 32] = bytes.try_into().expect("length checked above");
    let eph_kp = Ed25519KeyPair::from_bytes(&seed).unwrap();

    // This value can be stored with secret-manager. To do that, follow the prompt `sh configure_enclave.sh`
    // Answer `y` to `Do you want to use a secret?` and finish.
    // Then uncomment this code instead to fetch from env var API_KEY, which is fetched from secret manager.
    // let api_key = "045a27812dbe456392913223221306".to_string();

    let pk_string = std::env::var("SUI_PK")
        .map_err(|_| anyhow::anyhow!("SUI_PK environment variable not set"))?;
    let state = Arc::new(AppState { eph_kp, pk_string });

    // Define your own restricted CORS policy here if needed.
    let cors = CorsLayer::new().allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/", get(ping))
        .route("/get_attestation", get(get_attestation))
        .route("/process_data_v2", post(process_data_v2))
        .route("/health_check", get(health_check))
        .route("/health_check_post", post(health_check_post))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}

async fn ping() -> &'static str {
    "Ping Pong!"
}

async fn health_check_post() -> &'static str {
    "Health check post!"
}