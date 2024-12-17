use crate::error::Error;
use async_convert::async_trait;
use async_dropper::{AsyncDrop, AsyncDropError};
use deadpool_tiberius::{deadpool::managed::Object, Client, Manager, Pool};

/// A wrapper for a SQLServer Transaction.
/// Calls 'BEGIN TRAN' when initialized from a connection,
/// and 'ROLLBACK' when dropped.
#[derive(Default, AsyncDrop)]
pub struct Transaction {
    connection: Option<Object<Manager>>,
}

impl Transaction {
    /// Begin a new transaction.
    pub async fn new(pool: &Pool) -> Result<Self, Error> {
        let mut connection = pool.get().await?;
        connection.simple_query("BEGIN TRAN").await?;

        Ok(Self {
            connection: Some(connection),
        })
    }

    /// Get the underlying connection.
    pub fn inner(&mut self) -> Result<&mut Client, Error> {
        self.connection
            .as_deref_mut()
            .ok_or(Error::MissingConnection)
    }

    /// 'COMMIT' the transaction, consuming self.
    pub async fn commit(mut self) -> Result<(), Error> {
        match self.connection.take() {
            Some(mut conn) => {
                conn.simple_query("COMMIT").await?;

                Ok(())
            }
            None => Err(Error::MissingConnection),
        }
    }

    /// 'ROLLBACK' the transaction, consuming self.
    pub async fn rollback(mut self) -> Result<(), Error> {
        match self.connection.take() {
            Some(mut conn) => {
                conn.simple_query("ROLLBACK").await?;

                Ok(())
            }
            None => Err(Error::MissingConnection),
        }
    }
}

#[async_trait]
impl AsyncDrop for Transaction {
    async fn async_drop(&mut self) -> Result<(), AsyncDropError> {
        if let Some(conn) = &mut self.connection {
            conn.simple_query("ROLLBACK")
                .await
                .expect("Unable to rollback transaction");
        }

        Ok(())
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.connection.is_none() && other.connection.is_none()
    }
}

impl Eq for Transaction {}
