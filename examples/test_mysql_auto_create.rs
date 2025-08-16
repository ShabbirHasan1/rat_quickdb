//! 测试MySQL自动建表功能
//! 
//! 这个测试程序验证MySQL适配器在插入数据时是否能正确处理id字段

use rat_quickdb::{
    config::{GlobalConfig, GlobalConfigBuilder, AppConfigBuilder, LoggingConfigBuilder, Environment, LogLevel},
    types::{DataValue, DatabaseType, PoolConfig, ConnectionConfig, DatabaseConfig},
    odm::{get_odm_manager, OdmOperations},
    manager::add_database,
    error::QuickDbResult,
    init,
};
use std::collections::HashMap;
use zerg_creep::{info, error};

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    init();
    
    info!("开始测试MySQL自动建表功能...");
    
    // 创建连接池配置
    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        connection_timeout: 30,
        idle_timeout: 600,
        max_lifetime: 1800,
    };
    
    // 创建MySQL数据库配置（注意：这需要实际的MySQL服务器）
    let db_config = DatabaseConfig {
        alias: "test_mysql_db".to_string(),
        db_type: DatabaseType::MySQL,
        connection: ConnectionConfig::MySQL {
            host: "localhost".to_string(),
            port: 3306,
            database: "test_db".to_string(),
            username: "root".to_string(),
            password: "password".to_string(),
            ssl_opts: None,
            tls_config: None,
        },
        pool: pool_config,
        id_strategy: rat_quickdb::types::IdStrategy::AutoIncrement,
        cache: None,
    };
    
    // 添加数据库配置到管理器
    add_database(db_config).await?;
    
    // 获取ODM管理器
    let odm = get_odm_manager().await;
    
    // 准备测试数据（包含id字段）
    let mut user_data = HashMap::new();
    user_data.insert("id".to_string(), DataValue::Int(100));
    user_data.insert("name".to_string(), DataValue::String("张三".to_string()));
    user_data.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));
    user_data.insert("age".to_string(), DataValue::Int(25));
    user_data.insert("is_active".to_string(), DataValue::Bool(true));
    
    info!("插入用户数据: {:?}", user_data);
    
    // 插入数据，这应该会自动创建表
    match odm.create("users", user_data, None).await {
        Ok(result) => {
            info!("用户创建成功，结果: {:?}", result);
        }
        Err(e) => {
            error!("用户创建失败: {:?}", e);
            // 如果是连接错误（比如MySQL服务器未运行），这是正常的
            if e.to_string().contains("连接") {
                info!("MySQL服务器未运行，跳过测试");
                return Ok(());
            }
            return Err(e);
        }
    }
    
    info!("MySQL自动建表功能测试完成！");
    
    Ok(())
}