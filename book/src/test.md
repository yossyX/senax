# テスト

テストはテストDB接続URLで指定されたDBで実行されます。  
start_test()の呼び出しでデータベースは初期化されマイグレーションが実行されます。  
実DBを使用しますので、排他処理により一つずつ実行されます。

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test() -> Result<()> {
        dotenvy::dotenv().ok();
        let _guard = db_sample::start_test().await?;
        let mut conn = DbConn::new();
        conn.begin().await?;
        ...
        conn.commit().await?;
        Ok(())
    }
}
```