use crate::capabilities::config;
use bcrypt::{hash, verify};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::{lib::common_error::CommonError, logger};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims<T> {
    pub payload: T,
    pub exp: i64,
}

/// Sign a JWT with a payload and duration
pub fn jwt_sign<T: Serialize + for<'b> Deserialize<'b>>(
    payload: &T,
    duration: i64,
    secret_key_var: Option<&str>,
) -> Result<String, CommonError> {
    let key_var = secret_key_var.unwrap_or("IAM_JWT_SECRET");
    let exp_result = Utc::now().checked_add_signed(chrono::Duration::seconds(duration));

    if exp_result.is_none() {
        return Err(CommonError::from("Error calculating expiry".to_owned()));
    }
    let expiry = exp_result.unwrap().timestamp();
    let claims = Claims {
        payload,
        exp: expiry,
    };
    let header = Header::new(Algorithm::HS512);
    let secret: String = config::get_env(key_var);
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    match encode(&header, &claims, &encoding_key) {
        Ok(jwt) => Ok(jwt),
        Err(e) => {
            logger::error(&format!("{}", e));
            Err(CommonError::from("Error encoding JWT".to_owned()))
        }
    }
}

/// Verify and get data from
pub fn jwt_verify<T: Serialize + for<'b> Deserialize<'b>>(
    jwt: &str,
    secret_key_var: Option<&str>,
) -> Result<T, CommonError> {
    let key_var = secret_key_var.unwrap_or("IAM_JWT_SECRET");
    let secret: String = config::get_env(key_var);

    let mut validation = Validation::new(Algorithm::HS512);
    // force validation of expiry
    validation.validate_exp = true;

    let decode_result = decode::<Claims<T>>(
        jwt,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    );

    match decode_result {
        Ok(data) => Ok(data.claims.payload),
        Err(e) => {
            logger::error(&format!("{}", e));
            if e.to_string() == "ExpiredSignature" {
                Err(CommonError::from("JWT expired".to_owned()))
            } else {
                Err(CommonError::from("Error decoding JWT".to_owned()))
            }
        }
    }
}

pub fn hash_password(password: &str) -> Result<String, CommonError> {
    let cost = config::get_env::<u32>("BCRYPT_SALT");
    match hash(password, cost) {
        Ok(hashed) => Ok(hashed),
        Err(e) => {
            logger::error(&format!("{}", e));
            Err(CommonError::from("Error hashing password".to_owned()))
        }
    }
}

pub fn verify_password(password: &str, hashed: &str) -> Result<bool, CommonError> {
    match verify(password, hashed) {
        Ok(result) => Ok(result),
        Err(e) => {
            logger::error(&format!("{}", e));
            Err(CommonError::from("Error verifying password".to_owned()))
        }
    }
}
