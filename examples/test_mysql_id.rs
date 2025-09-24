use rat_quickdb::{
    types::*,
    odm::AsyncOdmManager,
    manager::{PoolManager, get_global_pool_manager},
    error::{QuickDbResult, QuickDbError},
    odm::OdmOperations,
    adapter::DatabaseAdapter,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio;
use rat_logger::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestUser {
    name: String,
    email: String,
}

impl TestUser {
    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut data = HashMap::new();
        data.insert("name".to_string(), DataValue::String(self.name.clone()));
        data.insert("email".to_string(), DataValue::String(self.email.clone()));
        data
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    rat_logger::LoggerBuilder::new().add_terminal_with_config(rat_logger::handler::term::TermConfig::default()).init().expect("日志初始化失败");
    
    info!("=== MySQL ID 返回测试 ===");
    
    // 创建数据库配置
    let pool_config = PoolConfig {
        min_connections: 1,
        max_connections: 5,
        connection_timeout: 30,
        idle_timeout: 600,
        max_lifetime: 3600,
    };

    let db_config = DatabaseConfig {
        alias: "mysql_test".to_string(),
        db_type: DatabaseType::MySQL,
        connection: ConnectionConfig::MySQL {
            host: "172.16.0.21".to_string(),
            port: 3306,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_opts: None,
            tls_config: None,
        },
        pool: pool_config,
        id_strategy: IdStrategy::AutoIncrement,
        cache: None,
    };

    // 添加数据库配置
    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;

    // 创建ODM管理器
    let odm = AsyncOdmManager::new();

    // 测试数据
    let test_user = TestUser {
        name: "测试用户".to_string(),
        email: "test@example.com".to_string(),
    };

    let table_name = "test_id_return";
    
    // 创建用户
    let data = test_user.to_data_map();
    info!("创建用户: {:?}", data);
    
    let created_id = odm.create(&table_name, data, Some("mysql_test")).await?;
    info!("返回的ID: {:?}", created_id);
    
    // 尝试查询刚插入的记录
    if let DataValue::Object(map) = &created_id {
        if let Some(DataValue::Int(id)) = map.get("id") {
            info!("解析到的ID: {}", id);
            
            // 查询验证
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int(*id),
            }];
            
            let results = odm.find(&table_name, conditions, None, Some("mysql_test")).await?;
            info!("查询结果数量: {}", results.len());
            
            if let Some(first_result) = results.first() {
                info!("查询结果: {:?}", first_result);
            }
        }
    }
    
    Ok(())
}