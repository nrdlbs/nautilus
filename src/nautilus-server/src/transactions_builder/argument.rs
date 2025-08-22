use anyhow::Result;
use std::collections::HashMap;
use sui_graphql_client::Client;
use sui_sdk_types::{Address, Argument, Identifier, TypeTag};
use sui_transaction_builder::{Serialized, TransactionBuilder, Function};

use crate::transactions_builder::helper;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectArgKind {
    Owned,
    Receiving,
    SharedRef,
    SharedMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutability {
    Immutable,
    Mutable,
}

#[derive(Default)]
pub struct ArgCache {
    object_args: HashMap<Address, (Argument, Mutability)>,
}

pub fn pure<Pure: serde::Serialize>(
    builder: &mut TransactionBuilder,
    value: Pure,
) -> Result<Argument> {
    let value_arg = builder.input(Serialized(&value));
    Ok(value_arg)
}

pub async fn owned(
    client: &Client,
    builder: &mut TransactionBuilder,
    id: Address,
) -> Result<Argument> {
    let object_input = helper::get_as_input(client, id).await?;
    let object_arg = builder.input(object_input.with_owned_kind());
    Ok(object_arg)
}

pub async fn owned_cached(
    client: &Client,
    builder: &mut TransactionBuilder,
    cache: &mut ArgCache,
    id: Address,
) -> Result<Argument> {
    if let Some(arg) = cache.object_args.get(&id) {
        return Ok(arg.0);
    }
    let arg = owned(client, builder, id).await?;
    cache
        .object_args
        .insert(id, (arg, Mutability::Immutable));
    Ok(arg)
}

pub async fn receiving(
    client: &Client,
    builder: &mut TransactionBuilder,
    id: Address,
) -> Result<Argument> {
    let object_input = helper::get_as_input(client, id).await?;
    let object_arg = builder.input(object_input.with_receiving_kind());
    Ok(object_arg)
}

pub async fn receiving_cached(
    client: &Client,
    builder: &mut TransactionBuilder,
    cache: &mut ArgCache,
    id: Address,
) -> Result<Argument> {
    if let Some(arg) = cache
        .object_args
        .get(&id)
    {
        return Ok(arg.0);
    }
    let arg = receiving(client, builder, id).await?;
    cache
        .object_args
        .insert(id, (arg, Mutability::Immutable));
    Ok(arg)
}

pub async fn shared_ref(
    client: &Client,
    builder: &mut TransactionBuilder,
    id: Address,
) -> Result<Argument> {
    let object_input = helper::get_as_input(client, id).await?;
    let object_arg = builder.input(object_input.by_ref());
    Ok(object_arg)
}

pub async fn shared_ref_cached(
    client: &Client,
    builder: &mut TransactionBuilder,
    cache: &mut ArgCache,
    id: Address,
) -> Result<Argument> {
    if let Some(arg) = cache
        .object_args
        .get(&id)
    {
        return Ok(arg.0);
    }
    let arg = shared_ref(client, builder, id).await?;
    cache
        .object_args
        .insert(id, (arg, Mutability::Immutable));
    Ok(arg)
}

pub async fn shared_mut(
    client: &Client,
    builder: &mut TransactionBuilder,
    id: Address,
) -> Result<Argument> {
    let object_input = helper::get_as_input(client, id).await?;
    let object_arg = builder.input(object_input.by_mut());
    Ok(object_arg)
}

pub async fn shared_mut_cached(
    client: &Client,
    builder: &mut TransactionBuilder,
    cache: &mut ArgCache,
    id: Address,
) -> Result<Argument> {
    if let Some(arg) = cache
        .object_args
        .get(&id)
    {
        return Ok(arg.0);
    }
    let arg = shared_mut(client, builder, id).await?;
    cache
        .object_args
        .insert(id, (arg, Mutability::Mutable));
    Ok(arg)
}

pub fn zero_coin(
    builder: &mut TransactionBuilder,
    coin_type: TypeTag,
) -> Result<Argument> {
    let zero_coin_arg = builder.move_call(
        Function::new(
            Address::from_hex("0x2").unwrap(),
            Identifier::new("coin").unwrap(),
            Identifier::new("zero").unwrap(),
            vec![coin_type],
        ),
        vec![],
    );
    Ok(zero_coin_arg)
}

pub fn destroy_zero_coin(
    builder: &mut TransactionBuilder,
    coin: Argument,
) -> Result<Argument> {
    let destroy_zero_coin_arg = builder.move_call(
        Function::new(
            Address::from_hex("0x2").unwrap(),
            Identifier::new("coin").unwrap(),
            Identifier::new("destroy_zero").unwrap(),
            vec![],
        ),
        vec![coin],
    );
    Ok(destroy_zero_coin_arg)
}