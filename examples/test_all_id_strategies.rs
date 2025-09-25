//! 测试所有IdStrategy策略
//!
//! 验证AutoIncrement, Uuid, Snowflake, ObjectId, Custom策略是否都正常工作

use rat_quickdb::{
    odm::{get_odm_manager, OdmOperations},
    types::{DatabaseConfig, DatabaseType, ConnectionConfig, PoolConfig, IdStrategy},
    manager::{add_database, get_global_pool_manager},
    DataValue,
};
use std::collections::HashMap;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试所有IdStrategy策略 ===\n");

    let pool_config = PoolConfig::builder()
        .max_connections(3)
        .min_connections(1)
        .connection_timeout(5000)
        .idle_timeout(300000)
        .max_lifetime(1800000)
        .build()?;

    let strategies = vec![
        ("auto_increment", IdStrategy::AutoIncrement),
        ("uuid", IdStrategy::Uuid),
        ("snowflake", IdStrategy::Snowflake { machine_id: 1, datacenter_id: 1 }),
        ("object_id", IdStrategy::ObjectId),
        ("custom", IdStrategy::Custom("test_prefix".to_string())),
    ];

    for (strategy_name, strategy) in strategies {
        println!("--- 测试策略: {} ---", strategy_name);

        let db_config = DatabaseConfig::builder()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite {
                path: format!("./test_{}.db", strategy_name),
                create_if_missing: true,
            })
            .pool(pool_config.clone())
            .alias(strategy_name)
            .id_strategy(strategy)
            .build()?;

        let pool_manager = get_global_pool_manager();
        pool_manager.add_database(db_config).await?;

        let odm_manager = get_odm_manager().await;

        // 清理测试数据
        let _ = odm_manager.delete("test_items", vec![], Some(strategy_name)).await;

        // 测试数据
        let mut test_data = HashMap::new();
        test_data.insert("name".to_string(), DataValue::String(format!("测试项目_{}", strategy_name)));
        test_data.insert("value".to_string(), DataValue::Int(42));

        match odm_manager.create("test_items", test_data, Some(strategy_name)).await {
            Ok(id_value) => {
                println!("✅ 创建成功，ID: {:?}", id_value);

                // 验证查询
                match &id_value {
                    DataValue::String(id_str) => {
                        match odm_manager.find_by_id("test_items", id_str, Some(strategy_name)).await {
                            Ok(Some(item)) => {
                                println!("✅ 查询成功: {}", item);
                            },
                            Ok(None) => println!("❌ 查询失败：未找到项目"),
                            Err(e) => println!("❌ 查询失败: {}", e),
                        }
                    },
                    DataValue::Int(id_int) => {
                        match odm_manager.find_by_id("test_items", &id_int.to_string(), Some(strategy_name)).await {
                            Ok(Some(item)) => {
                                println!("✅ 查询成功: {}", item);
                            },
                            Ok(None) => println!("❌ 查询失败：未找到项目"),
                            Err(e) => println!("❌ 查询失败: {}", e),
                        }
                    },
                    _ => println!("❌ 未知的ID类型: {:?}", id_value),
                }

                // 验证ID格式符合预期
                match strategy_name {
                    "auto_increment" => {
                        if let DataValue::Int(id_int) = id_value {
                            if id_int > 0 {
                                println!("✅ AutoIncrement ID格式正确: {}", id_int);
                            } else {
                                println!("❌ AutoIncrement ID格式错误: {}", id_int);
                            }
                        } else {
                            println!("❌ AutoIncrement应该是整数类型");
                        }
                    },
                    "uuid" => {
                        if let DataValue::String(id_str) = &id_value {
                            if uuid::Uuid::parse_str(id_str).is_ok() {
                                println!("✅ UUID格式正确: {}", id_str);
                            } else {
                                println!("❌ UUID格式错误: {}", id_str);
                            }
                        } else {
                            println!("❌ UUID应该是字符串类型");
                        }
                    },
                    "snowflake" => {
                        if let DataValue::String(id_str) = &id_value {
                            if id_str.parse::<u64>().is_ok() {
                                println!("✅ Snowflake ID格式正确: {}", id_str);
                            } else {
                                println!("❌ Snowflake ID格式错误: {}", id_str);
                            }
                        } else {
                            println!("❌ Snowflake ID应该是字符串类型");
                        }
                    },
                    "object_id" => {
                        if let DataValue::String(id_str) = &id_value {
                            if id_str.len() == 24 && id_str.chars().all(|c| c.is_ascii_hexdigit()) {
                                println!("✅ ObjectId格式正确: {}", id_str);
                            } else {
                                println!("❌ ObjectId格式错误: {}", id_str);
                            }
                        } else {
                            println!("❌ ObjectId应该是字符串类型");
                        }
                    },
                    "custom" => {
                        if let DataValue::String(id_str) = &id_value {
                            if id_str.starts_with("test_prefix_") {
                                println!("✅ Custom ID格式正确: {}", id_str);
                            } else {
                                println!("❌ Custom ID格式错误: {}", id_str);
                            }
                        } else {
                            println!("❌ Custom ID应该是字符串类型");
                        }
                    },
                    _ => println!("❌ 未知的策略名称: {}", strategy_name),
                }
            },
            Err(e) => {
                println!("❌ 创建失败: {}", e);
            }
        }

        println!();
    }

    println!("=== 所有策略测试完成 ===");
    Ok(())
}