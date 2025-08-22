use num_bigint::{BigUint, ToBigUint};
use num_traits::cast::ToPrimitive;

pub fn mul_div_floor(num1: u128, num2: u128, denom: u128) -> u128 {
        let r = full_mul(num1, num2) / (BigUint::from(denom));
        r.to_u128().unwrap()
    }

    pub fn mul_div_round(num1: u128, num2: u128, denom: u128) -> u128 {
        let r: BigUint = (full_mul(num1, num2) + (BigUint::from(denom) >> 1)) / (BigUint::from(denom));
        r.to_u128().unwrap()
    }

    pub fn mul_div_ceil(num1: u128, num2: u128, denom: u128) -> u128 {
        let r: BigUint = (full_mul(num1, num2) + (BigUint::from(denom) - 1u32)) / (BigUint::from(denom));
        r.to_u128().unwrap()
    }

    pub fn mul_shr(num1: u128, num2: u128, shift: u8) -> u128 {
        let product = full_mul(num1, num2) >> shift;
        product.to_u128().unwrap()
    }

    pub fn mul_shl(num1: u128, num2: u128, shift: u8) -> u128 {
        let product = full_mul(num1, num2) << shift;
        product.to_u128().unwrap()
    }

    pub fn full_mul(num1: u128, num2: u128) -> BigUint {
        (BigUint::from(num1)) * (BigUint::from(num2))
    }