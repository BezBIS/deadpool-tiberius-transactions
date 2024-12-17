#[cfg(test)]
mod test {
    use crate::Transaction;
    use deadpool_tiberius::{Manager, Pool};
    use dotenvy_macro::dotenv;

    fn pool() -> Pool {
        Manager::new()
            .host(dotenv!("HOST"))
            .port(dotenv!("PORT").parse::<u16>().unwrap())
            .basic_authentication(dotenv!("USER"), dotenv!("PASSWORD"))
            .database(dotenv!("DATABASE"))
            .trust_cert()
            .create_pool()
            .expect("Error establishing a connection pool.")
    }

    #[tokio::test]
    async fn test_commit() {
        let pool = pool();

        // Create table
        let mut conn = pool.get().await.unwrap();
        conn.simple_query("CREATE TABLE test_commit(id int);")
            .await
            .unwrap();

        // Create transaction and insert data.
        let mut tran = Transaction::new(&pool).await.unwrap();
        tran.inner()
            .unwrap()
            .simple_query(
                r#"
                INSERT INTO test_commit(id)
                VALUES (1);"#,
            )
            .await
            .unwrap();

        // Commit transaction
        let _ = tran.commit().await.unwrap();

        // Check data has inserted.
        let rows = conn
            .simple_query("SELECT * FROM test_commit;")
            .await
            .unwrap()
            .into_first_result()
            .await
            .unwrap();

        assert_eq!(rows[0].get(0), Some(1));

        // Finally, delete temp table.
        conn.simple_query("DROP TABLE test_commit;").await.unwrap();
    }

    #[tokio::test]
    async fn test_simple_rollback() {
        let pool = pool();

        // Create table
        let mut conn = pool.get().await.unwrap();
        conn.simple_query("CREATE TABLE test_simple_rollback(id int);")
            .await
            .unwrap();

        // Create transaction and insert data.
        let mut tran = Transaction::new(&pool).await.unwrap();
        tran.inner()
            .unwrap()
            .simple_query(
                r#"
                INSERT INTO test_simple_rollback(id)
                VALUES (1);"#,
            )
            .await
            .unwrap();

        // Rollback transaction
        let _ = tran.rollback().await.unwrap();

        // Check test table is empty.
        let rows = conn
            .simple_query("SELECT * FROM test_simple_rollback;")
            .await
            .unwrap()
            .into_first_result()
            .await
            .unwrap();

        assert_eq!(rows.len(), 0);

        // Finally, delete temp table.
        conn.simple_query("DROP TABLE test_simple_rollback;")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_complex_rollback() {
        let pool = pool();

        // Create table
        let mut conn = pool.get().await.unwrap();
        conn.simple_query("CREATE TABLE test_complex_rollback(id int);")
            .await
            .unwrap();

        // Create transaction and insert data.
        conn.simple_query(
            r#"
                INSERT INTO test_complex_rollback(id)
                VALUES (1);"#,
        )
        .await
        .unwrap();
        let mut tran = Transaction::new(&pool).await.unwrap();
        tran.inner()
            .unwrap()
            .simple_query(
                r#"
                INSERT INTO test_complex_rollback(id)
                VALUES (2);"#,
            )
            .await
            .unwrap();
        tran.inner()
            .unwrap()
            .simple_query(
                r#"
                INSERT INTO test_complex_rollback(id)
                VALUES (3);"#,
            )
            .await
            .unwrap();

        // Rollback transaction
        let _ = tran.rollback().await.unwrap();

        // Check test table only has first row.
        let rows = conn
            .simple_query("SELECT * FROM test_complex_rollback;")
            .await
            .unwrap()
            .into_first_result()
            .await
            .unwrap();

        assert_eq!(rows[0].get(0), Some(1));

        // Finally, delete temp table.
        conn.simple_query("DROP TABLE test_complex_rollback;")
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_simple_drop() {
        let pool = pool();

        // Create table
        let mut conn = pool.get().await.unwrap();
        conn.simple_query("CREATE TABLE test_simple_drop(id int);")
            .await
            .unwrap();

        {
            // Create transaction and insert data.
            let mut tran = Transaction::new(&pool).await.unwrap();
            tran.inner()
                .unwrap()
                .simple_query(
                    r#"
                INSERT INTO test_simple_drop(id)
                VALUES (1);"#,
                )
                .await
                .unwrap();

            // Transaction should rollback when dropped.
        }

        // Check test table is empty.
        let rows = conn
            .simple_query("SELECT * FROM test_simple_drop;")
            .await
            .unwrap()
            .into_first_result()
            .await
            .unwrap();

        assert_eq!(rows.len(), 0);

        // Finally, delete temp table.
        conn.simple_query("DROP TABLE test_simple_drop;")
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_complex_drop() {
        let pool = pool();

        // Create table
        let mut conn = pool.get().await.unwrap();
        conn.simple_query("CREATE TABLE test_complex_drop(id int);")
            .await
            .unwrap();

        // Create transaction and insert data.
        let mut tran = Transaction::new(&pool).await.unwrap();
        tran.inner()
            .unwrap()
            .simple_query(
                r#"
                INSERT INTO test_complex_drop(id)
                VALUES (1);"#,
            )
            .await
            .unwrap();

        if let Some(tran) = failing_transaction(tran).await.ok() {
            tran.commit().await.unwrap();
        }

        // Check test table is empty.
        let rows = conn
            .simple_query("SELECT * FROM test_complex_drop;")
            .await
            .unwrap()
            .into_first_result()
            .await
            .unwrap();

        assert_eq!(rows.len(), 0);

        // Finally, delete temp table.
        conn.simple_query("DROP TABLE test_complex_drop;")
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dropped_connection() {
        let original_pool = pool();

        // Create table
        let mut conn = original_pool.get().await.unwrap();
        conn.simple_query("CREATE TABLE test_dropped_connection(id int);")
            .await
            .unwrap();

        // Create pool and transaction, and insert data.
        {
            let pool = pool();
            let mut tran = Transaction::new(&pool).await.unwrap();
            tran.inner()
                .unwrap()
                .simple_query(
                    r#"
                    INSERT INTO test_dropped_connection(id)
                    VALUES (1);"#,
                )
                .await
                .unwrap();
        }

        // Check test table is empty.
        let rows = conn
            .simple_query("SELECT * FROM test_dropped_connection;")
            .await
            .unwrap()
            .into_first_result()
            .await
            .unwrap();

        assert_eq!(rows.len(), 0);

        // Finally, delete temp table.
        conn.simple_query("DROP TABLE test_dropped_connection;")
            .await
            .unwrap();
    }

    async fn failing_transaction(mut transaction: Transaction) -> Result<Transaction, String> {
        transaction
            .inner()
            .unwrap()
            .simple_query(
                r#"
                INSERT INTO test_complex_drop(id)
                VALUES (2);"#,
            )
            .await
            .unwrap();

        Err("Aborting early.".to_string())
    }
}
