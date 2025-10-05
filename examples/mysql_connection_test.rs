use rat_quickdb::{
    types::*,
    manager::{PoolManager, get_global_pool_manager},
    error::QuickDbResult,
};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("测试连接MySQL...");

    let db_config = DatabaseConfig {
        db_type: DatabaseType::MySQL,
        connection: ConnectionConfig::MySQL {
            host: "172.16.0.21".to_string(),
            port: 3306,
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            database: "testdb".to_string(),
            tls_config: None,
        },
        pool: PoolConfig::default(),
        alias: "mysql_test".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };

    let pool_manager = get_global_pool_manager();

    match timeout(Duration::from_secs(10), pool_manager.add_database(db_config)).await {
        Ok(Ok(())) => println!("✅ 连接成功"),
        Ok(Err(e)) => println!("❌ 连接失败: {}", e),
        Err(_) => println!("❌ 连接超时"),
    }

    Ok(())
}