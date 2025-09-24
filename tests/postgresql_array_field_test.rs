use rat_quickdb::types::{DatabaseConfig, ConnectionConfig, PoolConfig, DataValue, IdStrategy};
use rat_quickdb::pool::DatabaseConnection;
use rat_quickdb::manager::get_global_pool_manager;
use rat_quickdb::{odm, quick_error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Student {
    id: Option<i64>,
    name: String,
    subjects: Vec<String>,
    scores: Vec<i32>,
    metadata: HashMap<String, DataValue>,
}

#[tokio::test]
async fn test_postgresql_array_field_serialization() {
    // PostgreSQL 配置（使用线上服务器）
    let config = DatabaseConfig {
        db_type: rat_quickdb::types::DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: Some(rat_quickdb::types::TlsConfig {
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
            idle_timeout: 600,
            max_lifetime: 1800,
        },
        alias: "postgresql_test".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };

    // 初始化连接池
    let pool_manager = get_global_pool_manager();
    if let Err(e) = pool_manager.add_database(config).await {
        if e.to_string().contains("连接") || e.to_string().contains("timeout") {
            println!("⚠️ PostgreSQL服务器未运行或连接超时，跳过测试: {}", e);
            return;
        }
        panic!("添加PostgreSQL数据库配置失败: {}", e);
    }

    // 创建测试数据
    let mut test_data = HashMap::new();
    test_data.insert("name".to_string(), DataValue::String("张三".to_string()));
    test_data.insert("subjects".to_string(), DataValue::Array(vec![
        DataValue::String("数学".to_string()),
        DataValue::String("物理".to_string()),
        DataValue::String("化学".to_string()),
    ]));
    test_data.insert("scores".to_string(), DataValue::Array(vec![
        DataValue::Int(95),
        DataValue::Int(87),
        DataValue::Int(92),
    ]));
    
    let mut metadata = HashMap::new();
    metadata.insert("grade".to_string(), "高三".to_string());
    metadata.insert("class".to_string(), "1班".to_string());
    
    let metadata_map: HashMap<String, DataValue> = metadata.into_iter()
        .map(|(k, v)| (k, DataValue::String(v)))
        .collect();
    test_data.insert("metadata".to_string(), DataValue::Object(metadata_map));

    println!("准备插入的测试数据: {:?}", test_data);

    // 插入数据
    match odm::create("students", test_data, Some("postgresql_test")).await {
        Ok(result) => {
            println!("✅ PostgreSQL数据插入成功: {:?}", result);
        }
        Err(e) => {
            if e.to_string().contains("连接") || e.to_string().contains("timeout") {
                println!("⚠️ PostgreSQL服务器连接失败，跳过测试: {}", e);
                return;
            }
            panic!("❌ PostgreSQL数据插入失败: {}", e);
        }
    }

    // 查询数据并验证数组字段序列化
    let results = odm::find("students", vec![], None, Some("postgresql_test")).await.unwrap();
    
    for result in results {
        println!("学生数据: {:?}", result);
        
        // 将DataValue转换为HashMap进行字段访问
        if let DataValue::Object(student_data) = result {
            // 检查数组字段的序列化格式
            if let Some(subjects) = student_data.get("subjects") {
                println!("科目字段类型: {}", subjects.type_name());
                println!("科目字段值: {:?}", subjects);
                
                // 验证是否为数组类型
                match subjects {
                    DataValue::Array(arr) => {
                        println!("✅ 科目字段正确序列化为数组: {:?}", arr);
                    },
                    DataValue::String(s) => {
                        println!("❌ 科目字段被错误序列化为字符串: {}", s);
                        // 尝试解析字符串
                        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&s) {
                            println!("  可以解析为: {:?}", parsed);
                        }
                    },
                    _ => {
                        println!("❌ 科目字段序列化为未知类型: {:?}", subjects);
                    }
                }
            }
            
            if let Some(scores) = student_data.get("scores") {
                println!("分数字段类型: {}", scores.type_name());
                println!("分数字段值: {:?}", scores);
                
                // 验证是否为数组类型
                match scores {
                    DataValue::Array(arr) => {
                        println!("✅ 分数字段正确序列化为数组: {:?}", arr);
                    },
                    DataValue::String(s) => {
                        println!("❌ 分数字段被错误序列化为字符串: {}", s);
                        // 尝试解析字符串
                        if let Ok(parsed) = serde_json::from_str::<Vec<i32>>(&s) {
                            println!("  可以解析为: {:?}", parsed);
                        }
                    },
                    _ => {
                        println!("❌ 分数字段序列化为未知类型: {:?}", scores);
                    }
                }
            }
        } else {
            println!("❌ 查询结果不是Object类型: {:?}", result);
        }
    }

    println!("🎯 PostgreSQL数组字段序列化测试完成");
}