use reqwest::Error as ReqError;
use serde_json::Error as SerError;
use std::num::{ParseIntError, TryFromIntError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OvhError {
    #[error("network issue")]
    Reqwest(#[from] ReqError),

    #[error("parseInt issue")]
    ParseIntError(#[from] ParseIntError),

    #[error("tryFromInt issue")]
    TryFromInt(#[from] TryFromIntError),

    #[error("serde issue")]
    Serde(#[from] SerError),

    #[error("generic error : `{0}`")]
    Generic(String),
}
