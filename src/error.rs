use thiserror::Error;

#[derive(Error, Debug)]
pub enum OvhError {

    #[error("network issue")]
    Reqwest,

    #[error("parseInt issue")]
    ParseIntError,

    #[error("tryFromInt issue")]
    TryFromInt,

    #[error("serde issue")]
    Serde,

    #[error("generic error : `{0}`")]
    Generic(String),

    #[error("unknown data store error")]
    Unknown,
}