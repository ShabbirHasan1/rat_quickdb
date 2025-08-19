//! PostgreSQL JSON 处理测试
//! 
//! 验证 PostgreSQL 适配器的 JSON/JSONB 字段处理是否正常工作

use rat_quickdb::types::*;
use rat_quickdb::manager::{get_global_pool_manager, add_database};
use rat_quickdb::odm::{AsyncOdmManager, OdmOperations};
use serde_json::json;
use std::collections::HashMap;
use zerg_creep::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    zerg_creep::init_logger();
    
    info!("开始 PostgreSQL JSON 测试");
    
    // 创建数据库配置 - 使用与pgsql_cache_performance_comparison.rs相同的配置
    let config = DatabaseConfig {
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: Some(TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
        },
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: "postgres_test".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    // 初始化数据库连接
    match add_database(config).await {
        Ok(_) => {
            println!("✅ PostgreSQL 连接成功");
        }
        Err(e) => {
            println!("⚠️ PostgreSQL 连接失败，跳过测试: {}", e);
            return Ok(());
        }
    };
    
    // 准备测试数据（移除手动设置的ID，让AutoIncrement策略自动生成）
    let mut test_data = HashMap::new();
    test_data.insert("name".to_string(), DataValue::String("张三".to_string()));
    
    // 测试 JSON 字段
    test_data.insert("profile".to_string(), DataValue::Json(json!({
        "age": 25,
        "city": "北京",
        "skills": ["Rust", "PostgreSQL", "JSONB"],
        "contact": {
            "email": "zhangsan@example.com",
            "phone": "13800138000"
        }
    })));
    
    // 测试嵌套 JSON 对象
    test_data.insert("metadata".to_string(), DataValue::Json(json!({
        "created_at": "2024-01-01T00:00:00Z",
        "tags": ["developer", "backend"],
        "preferences": {
            "theme": "dark",
            "language": "zh-CN"
        }
    })));
    
    println!("准备插入的测试数据: {:?}", test_data);
    
    // 创建ODM管理器
    let odm = AsyncOdmManager::new();
    
    // 1. 测试插入 JSON 数据
    println!("\n1. 测试插入 JSON 数据...");
    let user_id = match odm.create("json_test_users", test_data, Some("postgres_test")).await {
        Ok(result) => {
            println!("✅ JSON 数据插入成功: {:?}", result);
            result
        }
        Err(e) => {
            println!("❌ JSON 数据插入失败: {}", e);
            return Err(e.into());
        }
    };
    
    // 2. 测试查询 JSON 数据
    println!("\n2. 测试查询 JSON 数据...");
    let results = match odm.find("json_test_users", vec![], None, Some("postgres_test")).await {
        Ok(results) => {
            println!("✅ JSON 数据查询成功，找到 {} 条记录", results.len());
            results
        }
        Err(e) => {
            println!("❌ JSON 数据查询失败: {}", e);
            return Err(e.into());
        }
    };
    
    // 3. 验证查询结果中的 JSON 字段
    println!("\n3. 验证查询结果中的 JSON 字段...");
    for (i, result) in results.iter().enumerate() {
        println!("记录 {}: {:?}", i + 1, result);
        
        if let DataValue::Object(obj) = result {
            // 检查 profile 字段 - PostgreSQL的JSON/JSONB字段会被转换为Object类型
            if let Some(profile) = obj.get("profile") {
                match profile {
                    DataValue::Object(profile_obj) => {
                        info!("  ✅ profile 字段类型正确: Object (来自PostgreSQL JSON/JSONB)");
                        
                        // 验证JSON内容是否正确
                        if let Some(DataValue::Int(age)) = profile_obj.get("age") {
                            if *age == 25 {
                                info!("  ✓ profile.age 内容正确: {}", age);
                            } else {
                                warn!("  ✗ profile.age 内容错误: {}", age);
                            }
                        }
                        if let Some(DataValue::String(city)) = profile_obj.get("city") {
                            if city == "北京" {
                                info!("  ✓ profile.city 内容正确: {}", city);
                            } else {
                                warn!("  ✗ profile.city 内容错误: {}", city);
                            }
                        }
                    }
                    _ => {
                        warn!("  ❌ profile 字段类型错误: {:?}, PostgreSQL JSON字段通常转换为Object类型", profile.type_name());
                    }
                }
            }
            
            // 检查 metadata 字段
            if let Some(metadata) = obj.get("metadata") {
                match metadata {
                    DataValue::Object(metadata_obj) => {
                        info!("  ✅ metadata 字段类型正确: Object (来自PostgreSQL JSON/JSONB)");
                        
                        // 验证JSON内容是否正确
                        if let Some(DataValue::String(created_at)) = metadata_obj.get("created_at") {
                            if created_at == "2024-01-01T00:00:00Z" {
                                info!("  ✓ metadata.created_at 内容正确: {}", created_at);
                            } else {
                                warn!("  ✗ metadata.created_at 内容错误: {}", created_at);
                            }
                        }
                    }
                    _ => {
                        warn!("  ❌ metadata 字段类型错误: {:?}, PostgreSQL JSON字段通常转换为Object类型", metadata.type_name());
                    }
                }
            }
        }
    }
    
    println!("\n✅ PostgreSQL JSON 处理测试完成！");
    
    Ok(())
}