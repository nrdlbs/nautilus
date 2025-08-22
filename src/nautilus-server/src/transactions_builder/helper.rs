use std::time::{Duration, Instant};
use sui_crypto::{ed25519::Ed25519PrivateKey, SuiSigner};
use sui_sdk_types::{Address, ExecutionStatus, TransactionEffects};
use sui_transaction_builder::{unresolved::Input, TransactionBuilder};
use thiserror::Error;
use cynic::QueryBuilder;
use sui_graphql_client::{
    query_types::{MoveValue, ObjectFilter, ObjectsQuery, ObjectsQueryArgs},
    Client, Direction, DynamicFieldOutput, PaginationFilter,
};
use sui_sdk_types::{framework::Coin, Object};
use base64ct::Encoding;

#[derive(Error, Debug)]
pub enum SuiUtilsError {
    #[error("GraphQL error: {0}")]
    GraphQL(String),
    #[error("Object not found: {0}")]
    ObjectNotFound(Address),
    #[error("Object contents not found: {0}")]
    ObjectContentsNotFound(String),
    #[error("No SUI coin with minimum budget found")]
    GasCoinNotFound,
    #[error("Error while building gas input")]
    InvalidGasInput,
    #[error("Could not get reference gas price")]
    ReferenceGasPriceError,
    #[error("Error while building transaction")]
    TransactionBuildingError(String),
    #[error("Error while signing transaction")]
    TransactionSigningError(String),
    #[error("Error while executing transaction")]
    TransactionExecutionError(String),
    #[error("Could not get transaction effects")]
    InvalidTransactionEffects,
}

impl From<sui_graphql_client::error::Error> for SuiUtilsError {
    fn from(err: sui_graphql_client::error::Error) -> Self {
        SuiUtilsError::GraphQL(err.to_string())
    }
}

impl From<Vec<cynic::GraphQlError>> for SuiUtilsError {
    fn from(errors: Vec<cynic::GraphQlError>) -> Self {
        SuiUtilsError::GraphQL(format!("{:?}", errors))
    }
}

pub type Result<T> = std::result::Result<T, SuiUtilsError>;

pub async fn new_with_gas(
    client: &Client,
    caller: Address,
    gas_budget: u64,
) -> Result<TransactionBuilder> {
    let mut builder = TransactionBuilder::new();
    // get all sui coins
    let sui_coins =
        get_owned_coins(client, caller, Some("0x2::coin::Coin<0x2::sui::SUI>")).await?;
    // find the coin with the minimum balance according to the budget
    let gas_coin = sui_coins
        .iter()
        .find(|c| c.balance() >= gas_budget)
        .ok_or(SuiUtilsError::GasCoinNotFound)?;
    // build the gas input from the coin
    let gas_input: Input = (&client
        .object(gas_coin.id().to_owned().into(), None)
        .await?
        .ok_or(SuiUtilsError::InvalidGasInput)?)
        .into();
    // get the reference gas price
    let gas_price = client
        .reference_gas_price(None)
        .await?
        .ok_or(SuiUtilsError::ReferenceGasPriceError)?;

    builder.add_gas_objects(vec![gas_input.with_owned_kind()]);
    builder.set_gas_price(gas_price);
    builder.set_gas_budget(gas_budget);
    builder.set_sender(caller);

    Ok(builder)
}

pub async fn execute_and_wait_for_effects(
    client: &Client,
    builder: TransactionBuilder,
    pk: &Ed25519PrivateKey,
    dry_run: bool,
    skip_checks: Option<bool>,
) -> Result<TransactionEffects> {
    let tx = builder
        .finish()
        .map_err(|e| SuiUtilsError::TransactionBuildingError(e.to_string()))?;
    let sig = pk
        .sign_transaction(&tx)
        .map_err(|e| SuiUtilsError::TransactionSigningError(e.to_string()))?;
    
    let _tx_bytes = base64ct::Base64::encode_string(&bcs::to_bytes(&tx).map_err(|e| SuiUtilsError::TransactionBuildingError(e.to_string()))?);

    println!("tx: {:?}", tx);

    if dry_run {
        println!("dry running");
        match client.dry_run_tx(&tx, skip_checks).await {
            Ok(result) => {
                println!("dry run result: {:?}", result);
                if let Some(effects) = result.effects {
                    let status = effects.status();
                    if status == &ExecutionStatus::Success {
                        println!("dry run success");
                    } else {
                        println!("dry run failed");
                        return Err(SuiUtilsError::TransactionExecutionError("Dry run failed".to_string()));
                    }
                } else {
                    return Err(SuiUtilsError::TransactionExecutionError("Dry run failed - no effects".to_string()));
                }
            }
            Err(e) => {
                println!("dry run error: {:?}", e);
                return Err(SuiUtilsError::TransactionExecutionError(format!("Dry run failed: {:?}", e)));
            }
        }
    }

    let effects = client.execute_tx(vec![sig], &tx).await?;
    // wait for the transaction to be finalized with timeout
    let timeout_duration: Duration = Duration::from_secs(30); // 30 seconds timeout
    let start_time = Instant::now();
    match effects {
        Some(effects) => {
            let status = effects.status();
            if status == &ExecutionStatus::Success {
                while client.transaction(tx.digest()).await?.is_none() {
                    if start_time.elapsed() > timeout_duration {
                        return Err(SuiUtilsError::TransactionExecutionError(
                            "Transaction finalization timeout".to_string()
                        ));
                    }   
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Ok(effects)
            } else {
                Err(SuiUtilsError::TransactionExecutionError(format!(
                    "Transaction failed: {:?}",
                    status
                )))
            }
        }
        None => Err(SuiUtilsError::InvalidTransactionEffects),
    }
}

pub async fn get(client: &Client, id: Address) -> Result<Object> {
    client
        .object(id, None)
        .await?
        .ok_or(SuiUtilsError::ObjectNotFound(id))
}

pub async fn get_as_input(client: &Client, id: Address) -> Result<Input> {
    let object = get(client, id).await?;
    let input = Input::from(&object);

    Ok(input)
}

pub async fn get_multi(client: &Client, mut ids: Vec<Address>) -> Result<Vec<Object>> {
    let mut objects = Vec::new();

    while !ids.is_empty() {
        let resp = client
            .objects(
                Some(ObjectFilter {
                    object_ids: Some(ids.split_off(ids.len().saturating_sub(50))),
                    ..Default::default()
                }),
                PaginationFilter::default(),
            )
            .await?;
        objects.extend(resp.data().iter().cloned());
    }

    Ok(objects)
}

// gets `MoveValue`s from sui-graphql-client (returning the fields in json)
pub async fn get_multi_with_fields(client: &Client, mut ids: Vec<Address>) -> Result<Vec<MoveValue>> {
    let mut move_values = Vec::new();

    while !ids.is_empty() {
        let operation = ObjectsQuery::build(ObjectsQueryArgs {
            filter: Some(ObjectFilter {
                object_ids: Some(ids.split_off(ids.len().saturating_sub(50))),
                ..Default::default()
            }),
            after: None,
            before: None,
            first: Some(50),
            last: None,
        });

        let response = client.run_query(&operation)
            .await
            .map_err(|e| SuiUtilsError::GraphQL(e.to_string()))?;

        if let Some(objects) = response.data {
            for object in objects.objects.nodes {
                let object_string = format!("{:?}", object);
                let move_value = object
                    .as_move_object
                    .and_then(|move_object| move_object.contents)
                    .ok_or(SuiUtilsError::ObjectContentsNotFound(object_string))?;
                move_values.push(move_value);
            }
        }
    }

    Ok(move_values)
}

pub async fn get_owned(
    client: &Client,
    owner: Address,
    type_: Option<&str>,
) -> Result<Vec<Object>> {
    let mut objects = Vec::new();
    let mut cursor = None;
    let mut has_next_page = true;

    while has_next_page {
        let filter = PaginationFilter {
            direction: Direction::Forward,
            cursor: cursor.clone(),
            limit: Some(50),
        };

        let resp = client
            .objects(
                Some(ObjectFilter {
                    owner: Some(owner),
                    type_,
                    object_ids: None,
                }),
                filter,
            )
            .await?;
        objects.extend(resp.data().iter().cloned());

        cursor = resp.page_info().end_cursor.clone();
        has_next_page = resp.page_info().has_next_page;
    }

    Ok(objects)
}

pub async fn get_owned_coins(
    client: &Client,
    owner: Address,
    type_: Option<&str>,
) -> Result<Vec<Coin<'static>>> {
    let mut coins = Vec::new();
    let mut cursor = None;
    let mut has_next_page = true;

    while has_next_page {
        let filter = PaginationFilter {
            direction: Direction::Forward,
            cursor: cursor.clone(),
            limit: Some(50),
        };

        let resp = client.coins(owner, type_, filter).await?;
        coins.extend(resp.data().iter().cloned());

        cursor = resp.page_info().end_cursor.clone();
        has_next_page = resp.page_info().has_next_page;
    }

    Ok(coins)
}

// gets `MoveValue`s from sui-graphql-client (returning the fields in json)
pub async fn get_owned_with_fields(
    client: &Client,
    owner: Address,
    type_: Option<&str>,
) -> Result<Vec<MoveValue>> {
    let mut move_values = Vec::new();
    let mut cursor = None;
    let mut has_next_page = true;

    while has_next_page {
        let operation = ObjectsQuery::build(ObjectsQueryArgs {
            after: cursor.as_deref(),
            before: None,
            filter: Some(ObjectFilter {
                owner: Some(owner),
                type_,
                ..Default::default()
            }),
            first: Some(50),
            last: None,
        });

        let response = client.run_query(&operation)
            .await
            .map_err(|e| SuiUtilsError::GraphQL(e.to_string()))?;

        if let Some(objects) = response.data {
            for object in objects.objects.nodes {
                let object_string = format!("{:?}", object);
                let move_value = object
                    .as_move_object
                    .and_then(|move_object| move_object.contents)
                    .ok_or(SuiUtilsError::ObjectContentsNotFound(object_string))?;
                move_values.push(move_value);
            }

            cursor = objects.objects.page_info.end_cursor;
            has_next_page = objects.objects.page_info.has_next_page;
        }
    }

    Ok(move_values)
}

pub async fn get_dynamic_fields(client: &Client, id: Address) -> Result<Vec<DynamicFieldOutput>> {
    let mut dfs = Vec::new();
    let mut cursor = None;
    let mut has_next_page = true;

    while has_next_page {
        let filter = PaginationFilter {
            cursor: cursor.clone(),
            ..PaginationFilter::default()
        };

        let resp = client.dynamic_fields(id, filter).await?;
        dfs.extend(resp.data().iter().cloned());

        cursor = resp.page_info().end_cursor.clone();
        has_next_page = resp.page_info().has_next_page;
    }

    Ok(dfs)
}

pub fn tick_to_i32(tick: u32) -> i32 {
    if tick >= 2u32.pow(31) {
        (tick as i32) - 2u32.pow(32) as i32
    } else {
        tick as i32
    }
}