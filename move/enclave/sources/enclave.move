// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// Permissionless registration of an enclave.

module enclave::enclave;

use std::bcs;
use std::string::String;
use sui::ed25519;
use sui::nitro_attestation::NitroAttestationDocument;

const EInvalidPCRs: u64 = 0;
const EInvalidConfigVersion: u64 = 1;

// The expected PCRs.
// - We only define the first 3 PCRs. One can define other
//   PCRs and/or fields (e.g. user_data) if necessary as part
//   of the config.
// - See https://docs.aws.amazon.com/enclaves/latest/user/set-up-attestation.html#where
//   for more information on PCRs.
public struct EnclaveConfig<phantom T> has key {
    id: UID,
    name: String,
    pcr0: vector<u8>, // Enclave image file
    pcr1: vector<u8>, // Enclave Kernel
    pcr2: vector<u8>, // Enclave application
    version: u64,
}

// A verified enclave instance, with its public key.
public struct Enclave<phantom T> has key {
    id: UID,
    pk: vector<u8>,
    config_version: u64,
}

// A capability to update the enclave config.
public struct Cap<phantom T> has key, store {
    id: UID,
}

// An intent message, used for wrapping enclave messages.
public struct IntentMessage<T: drop> has copy, drop {
    intent: u8,
    timestamp_ms: u64,
    payload: T,
}

fun create_intent_message<P: drop>(intent: u8, timestamp_ms: u64, payload: P): IntentMessage<P> {
    IntentMessage {
        intent,
        timestamp_ms,
        payload,
    }
}

public fun create_enclave_config<T: drop>(
    _witness: T,
    name: String,
    pcr0: vector<u8>,
    pcr1: vector<u8>,
    pcr2: vector<u8>,
    ctx: &mut TxContext,
): Cap<T> {
    let enclave_config = EnclaveConfig<T> {
        id: object::new(ctx),
        name,
        pcr0,
        pcr1,
        pcr2,
        version: 0,
    };

    let cap = Cap {
        id: object::new(ctx),
    };

    transfer::share_object(enclave_config);
    cap
}

public fun register_enclave<T>(
    enclave_config: &EnclaveConfig<T>,
    document: NitroAttestationDocument,
    ctx: &mut TxContext,
) {
    let pk = load_pk(document, enclave_config);
    let enclave = Enclave<T> {
        id: object::new(ctx),
        pk,
        config_version: enclave_config.version,
    };
    transfer::share_object(enclave);
}

fun load_pk<T>(document: NitroAttestationDocument, enclave_config: &EnclaveConfig<T>): vector<u8> {
    let pcrs = document.pcrs();
    assert!(pcrs[0].index() == 0, EInvalidPCRs);
    assert!(pcrs[1].index() == 1, EInvalidPCRs);
    assert!(pcrs[2].index() == 2, EInvalidPCRs);
    assert!(pcrs[0].value() == enclave_config.pcr0, EInvalidPCRs);
    assert!(pcrs[1].value() == enclave_config.pcr1, EInvalidPCRs);
    assert!(pcrs[2].value() == enclave_config.pcr2, EInvalidPCRs);

    option::destroy_some(*document.public_key())
}

public fun verify_signature<T, P: drop>(
    enclave: &Enclave<T>,
    intent_scope: u8,
    timestamp_ms: u64,
    payload: P,
    signature: &vector<u8>,
): bool {
    let intent_message = create_intent_message(intent_scope, timestamp_ms, payload);
    let payload = bcs::to_bytes(&intent_message);
    return ed25519::ed25519_verify(signature, &enclave.pk, &payload)
}

public fun update_pcrs<T: drop>(
    config: &mut EnclaveConfig<T>,
    _cap: &Cap<T>,
    pcr0: vector<u8>,
    pcr1: vector<u8>,
    pcr2: vector<u8>,
) {
    config.pcr0 = pcr0;
    config.pcr1 = pcr1;
    config.pcr2 = pcr2;
    config.version = config.version + 1;
}

public fun update_name<T: drop>(config: &mut EnclaveConfig<T>, _cap: &Cap<T>, name: String) {
    config.name = name;
}

public fun pcr0<T>(config: &EnclaveConfig<T>): &vector<u8> {
    &config.pcr0
}

public fun pcr1<T>(config: &EnclaveConfig<T>): &vector<u8> {
    &config.pcr1
}

public fun pcr2<T>(config: &EnclaveConfig<T>): &vector<u8> {
    &config.pcr2
}

public fun pk<T>(enclave: &Enclave<T>): &vector<u8> {
    &enclave.pk
}

public fun destroy_old_enclave<T>(e: Enclave<T>, config: &EnclaveConfig<T>) {
    assert!(e.config_version < config.version, EInvalidConfigVersion);
    let Enclave { id, .. } = e;
    object::delete(id);
}

#[test_only]
public fun destroy_cap<T>(c: Cap<T>) {
    let Cap { id, .. } = c;
    object::delete(id);
}

#[test_only]
public fun destroy_enclave<T>(e: Enclave<T>) {
    let Enclave { id, .. } = e;
    object::delete(id);
}

#[test_only]
public struct SigningPayload has copy, drop {
    location: String,
    temperature: u64,
}

#[test]
fun test_serde() {
    // serialization should be consistent with rust test see `fn test_serde` in `src/nautilus-server/app.rs`.
    use std::string;

    let scope = 0;
    let timestamp = 1744038900000;
    let signing_payload = create_intent_message(
        scope,
        timestamp,
        SigningPayload {
            location: string::utf8(b"San Francisco"),
            temperature: 13,
        },
    );
    let bytes = bcs::to_bytes(&signing_payload);
    assert!(bytes == x"0020b1d110960100000d53616e204672616e636973636f0d00000000000000", 0);
}
