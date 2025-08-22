use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};

#[derive(Debug, Deserialize, Serialize)]
pub struct Route {
    pub code: u32,
    pub msg: String,
    pub data: Option<RouteData>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouteData {
    pub routes: Vec<SwapRoute>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SwapRoute {
    pub path: Vec<SwapPath>,
    pub amount_in: u64,
    pub amount_out: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SwapPath {
    pub id: String,
    pub provider: String,
    pub from: String,
    pub target: String,
    pub direction: bool,
    pub fee_rate: String,
    pub lot_size: u64,
    pub amount_in: u64,
    pub amount_out: u64,
}

pub struct SwapByAmountInRequest {
    pub from: String,
    pub to: String,
    pub amount_in: u64,
}

pub struct CetusAggregator;

impl CetusAggregator {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn swap_by_amount_in(&self, swap_request: SwapByAmountInRequest) -> Result<Route, Error> {
        let api_url = format!("https://api-sui.cetus.zone/router_v2/find_routes?from={}&target={}&amount={}&byAmountIn=true&depth=3&providers=CETUS&v={}", swap_request.from, swap_request.to, swap_request.amount_in, 1001600);
        let response = reqwest::get(&api_url).await.unwrap();
        let body = response.text().await.unwrap();

        // Parse response trực tiếp thành Route struct (không phải Vec<Route>)
        let route: Route = serde_json::from_str(&body).unwrap();
        if route.code != 200 {
            return Err(Error::new(ErrorKind::Other, route.msg));
        }

        Ok(route)
    }
    
    pub async fn get_mark_price(&self, coin_a_type: String, coin_b_type: String, amount: u64) -> Result<u128, Error> {
        let api_url = format!("https://api-sui.cetus.zone/router_v2/find_routes?from={}&target={}&amount={}&byAmountIn=true&depth=3&providers=CETUS&v=1001600", coin_a_type, coin_b_type, amount);
        let response = reqwest::get(&api_url).await.unwrap();
        let body = response.text().await.unwrap();

        let route: Route = serde_json::from_str(&body).unwrap();
        if route.code != 200 {
            return Err(Error::new(ErrorKind::Other, route.msg));
        }

        let sum_amount_out = route.data.as_ref().unwrap().routes.iter().map(|route| route.amount_out).sum::<u64>();
        let sum_amount_in = route.data.as_ref().unwrap().routes.iter().map(|route| route.amount_in).sum::<u64>();

        Ok(sum_amount_out as u128 * 1000000000u128 / sum_amount_in as u128)
    }
}