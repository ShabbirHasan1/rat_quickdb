//! MongoDB数据结构测试示例
//!
//! 本示例用于验证MongoDB数据是否被包裹在"Object"结构中
//! 包含完整的TLS配置和基本的CRUD操作测试

use rat_quickdb::{
    config::{mongodb_config_with_builder},
    error::QuickDbError,
    types::{
        DatabaseConfig, MongoDbConnectionBuilder, PoolConfig, TlsConfig, ZstdConfig,
        DataValue, QueryCondition, QueryOperator, QueryOptions
    },
    manager::{add_database, get_global_pool_manager},
    odm::{get_odm_manager, OdmOperations},
    init,
};
use std::collections::HashMap;
use zerg_creep::{info, error, debug};

#[tokio::main]
async fn main() -> Result<(), QuickDbError> {
    // 初始化日志系统
    zerg_creep::init_logger();
    init();
    
    info!("开始MongoDB数据结构测试");
    
    // 配置MongoDB连接（包含完整的TLS配置）
    let config = create_mongodb_config()?;
    
    // 添加数据库配置
    add_database(config).await?;
    
    // 获取ODM管理器
    let odm = get_odm_manager().await;
    
    let table_name = "test_data_structure";
    let alias = Some("mongodb_test");
    
    // 清理现有数据
    info!("清理现有测试数据");
    if let Err(e) = odm.delete(table_name, vec![], alias).await {
        debug!("清理现有数据失败（可能不存在）: {}", e);
    }
    
    // 创建测试数据
    info!("插入测试数据");
    let test_data = create_test_data();
    
    let create_result = odm.create(table_name, test_data.clone(), alias).await?;
    info!("数据插入结果: {:?}", create_result);
    
    // 查询所有数据
    info!("查询所有数据");
    let all_data = odm.find(
        table_name,
        vec![], // 空条件查询所有
        Some(QueryOptions::default()),
        alias
    ).await?;
    
    info!("查询结果: {:?}", all_data);
    
    // 检查数据结构
    analyze_data_structure(&all_data);
    
    // 按条件查询
    info!("按name条件查询");
    let name_condition = vec![QueryCondition {
        field: "name".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("张三".to_string()),
    }];
    
    let name_result = odm.find(
        table_name,
        name_condition,
        Some(QueryOptions::default()),
        alias
    ).await?;
    
    info!("按name查询结果: {:?}", name_result);
    analyze_data_structure(&name_result);
    
    // 按age条件查询
    info!("按age条件查询");
    let age_condition = vec![QueryCondition {
        field: "age".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::Int(25),
    }];
    
    let age_result = odm.find(
        table_name,
        age_condition,
        Some(QueryOptions::default()),
        alias
    ).await?;
    
    info!("按age查询结果: {:?}", age_result);
    analyze_data_structure(&age_result);
    
    // 清理测试数据
    info!("清理测试数据");
    if let Err(e) = odm.delete(table_name, vec![], alias).await {
        error!("清理测试数据失败: {}", e);
    }
    
    info!("MongoDB数据结构测试完成");
    Ok(())
}

/// 创建MongoDB配置（包含完整的TLS配置）
fn create_mongodb_config() -> Result<DatabaseConfig, QuickDbError> {
    let pool_config = PoolConfig::builder()
        .max_connections(10)
        .min_connections(2)
        .connection_timeout(5) // 减少超时时间
        .idle_timeout(300)
        .max_lifetime(1800)
        .build()?;
    
    // 配置TLS
    let tls_config = TlsConfig {
        enabled: true,
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
        verify_server_cert: false,
        verify_hostname: false,
        min_tls_version: None,
        cipher_suites: None,
    };
    
    // 配置ZSTD压缩
    let zstd_config = ZstdConfig {
        enabled: true,
        compression_level: Some(3),
        compression_threshold: Some(1024),
    };
    
    // 使用构建器创建连接配置
    let builder = MongoDbConnectionBuilder::new("db0.0ldm0s.net", 27017, "testdb")
        .with_auth("testdb", "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^")
        .with_auth_source("testdb")
        .with_direct_connection(true)
        .with_tls_config(tls_config)
        .with_zstd_config(zstd_config)
        .with_option("retryWrites", "true")
        .with_option("w", "majority");
    
    mongodb_config_with_builder(
        "mongodb_test",
        builder,
        pool_config,
    )
}

/// 创建测试数据
fn create_test_data() -> HashMap<String, DataValue> {
    let mut data = HashMap::new();
    data.insert("id".to_string(), DataValue::String("user_001".to_string()));
    data.insert("name".to_string(), DataValue::String("张三".to_string()));
    data.insert("age".to_string(), DataValue::Int(25));
    data.insert("city".to_string(), DataValue::String("北京".to_string()));
    data.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));
    data.insert("is_active".to_string(), DataValue::Bool(true));
    data
}

/// 分析数据结构
fn analyze_data_structure(data: &Vec<DataValue>) {
    info!("=== 数据结构分析 ===");
    info!("查询结果数量: {}", data.len());
    
    for (index, record) in data.iter().enumerate() {
        // 显示原始的 Debug 格式
        info!("记录 {} (Debug格式): {:?}", index + 1, record);
        
        // 显示 JSON 格式
        let json_value = record.to_json_value();
        info!("记录 {} (JSON格式): {}", index + 1, serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "JSON序列化失败".to_string()));
        
        // 检查数据结构类型
        match record {
            DataValue::Object(obj) => {
                info!("✅ 正常Object结构: 记录 {}", index + 1);
                // 分析Object内部结构，检查是否有嵌套的Object包装
                for (key, value) in obj {
                    match value {
                        DataValue::Object(nested_obj) => {
                            error!("❌ 发现嵌套Object包装! 字段: {}, 内容: {:?}", key, nested_obj);
                            // 进一步检查嵌套Object的内容
                            for (nested_key, nested_value) in nested_obj {
                                debug!("    嵌套Object内字段: {} = {:?}", nested_key, nested_value);
                            }
                        },
                        _ => {
                            debug!("  ✅ 正常字段: {} = {:?}", key, value);
                        }
                    }
                }
            },
            _ => {
                info!("⚠️ 非Object数据类型: {:?}", record);
            }
        }
    }
    info!("=== 数据结构分析完成 ===");
}