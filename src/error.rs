use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Tiberius(#[from] tiberius::error::Error),
    #[error("{0}")]
    Pool(#[from] deadpool_tiberius::deadpool::managed::PoolError<tiberius::error::Error>),
    #[error("Transaction missing an associated connection.")]
    MissingConnection,
}
