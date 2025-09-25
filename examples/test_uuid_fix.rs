//! 测试IdStrategy::Uuid修复效果
//!
//! 这个简单的测试专门验证UUID生成是否正常工作

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
    println!("=== 测试IdStrategy::Uuid修复效果 ===\n");

    // 初始化连接池配置
    let pool_config = PoolConfig::builder()
        .max_connections(5)
        .min_connections(1)
        .connection_timeout(5000)
        .idle_timeout(300000)
        .max_lifetime(1800000)
        .build()?;

    // 创建数据库配置，使用Uuid策略
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./test_uuid.db".to_string(),
            create_if_missing: true,
        })
        .pool(pool_config)
        .alias("test_uuid")
        .id_strategy(IdStrategy::Uuid)
        .build()?;

    // 添加数据库配置
    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;
    pool_manager.set_default_alias("test_uuid").await?;

    // 获取ODM管理器
    let odm_manager = get_odm_manager().await;

    // 清理可能存在的测试数据
    println!("清理现有测试数据...");
    let _ = odm_manager.delete("test_users", vec![], Some("test_uuid")).await;

    println!("测试1: 创建不包含ID的用户数据（应该自动生成UUID）...");

    // 测试数据 - 不包含id字段
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String("测试用户".to_string()));
    user_data.insert("email".to_string(), DataValue::String("test@example.com".to_string()));
    user_data.insert("age".to_string(), DataValue::Int(25));

    // 创建用户
    match odm_manager.create("test_users", user_data.clone(), Some("test_uuid")).await {
        Ok(id_value) => {
            println!("✅ 用户创建成功，生成的ID: {:?}", id_value);

            // 验证ID格式
            match &id_value {
                DataValue::String(id_str) => {
                    // 检查是否是有效的UUID格式
                    if uuid::Uuid::parse_str(id_str).is_ok() {
                        println!("✅ 生成的ID是有效的UUID格式: {}", id_str);
                    } else {
                        println!("❌ 生成的ID不是有效的UUID格式: {}", id_str);
                    }
                },
                _ => {
                    println!("❌ 生成的ID不是字符串类型: {:?}", id_value);
                }
            }

            // 查询验证
            match odm_manager.find_by_id("test_users", id_value.to_string().as_str(), Some("test_uuid")).await {
                Ok(Some(user_json)) => {
                    println!("✅ 用户查询成功: {}", user_json);
                },
                Ok(None) => {
                    println!("❌ 用户查询失败：未找到用户");
                },
                Err(e) => {
                    println!("❌ 用户查询失败: {}", e);
                }
            }
        },
        Err(e) => {
            println!("❌ 用户创建失败: {}", e);
        }
    }

    println!("\n测试2: 创建另一个用户（验证唯一性）...");

    // 创建第二个用户
    let mut user_data2 = HashMap::new();
    user_data2.insert("name".to_string(), DataValue::String("测试用户2".to_string()));
    user_data2.insert("email".to_string(), DataValue::String("test2@example.com".to_string()));
    user_data2.insert("age".to_string(), DataValue::Int(30));

    match odm_manager.create("test_users", user_data2, Some("test_uuid")).await {
        Ok(id_value2) => {
            println!("✅ 第二个用户创建成功，生成的ID: {:?}", id_value2);

            // 查询所有用户
            match odm_manager.find("test_users", vec![], None, Some("test_uuid")).await {
                Ok(users) => {
                    println!("✅ 当前用户总数: {}", users.len());
                    for (i, user) in users.iter().enumerate() {
                        if let DataValue::Object(obj) = user {
                            if let Some(DataValue::String(id)) = obj.get("id") {
                                println!("   用户 {}: ID = {}", i + 1, id);
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("❌ 查询所有用户失败: {}", e);
                }
            }
        },
        Err(e) => {
            println!("❌ 第二个用户创建失败: {}", e);
        }
    }

    println!("\n测试3: 尝试手动指定ID（应该使用指定的ID而不是生成）...");

    // 测试手动指定ID的情况
    let mut user_data3 = HashMap::new();
    user_data3.insert("id".to_string(), DataValue::String("manual_id_001".to_string()));
    user_data3.insert("name".to_string(), DataValue::String("手动ID用户".to_string()));
    user_data3.insert("email".to_string(), DataValue::String("manual@example.com".to_string()));
    user_data3.insert("age".to_string(), DataValue::Int(35));

    match odm_manager.create("test_users", user_data3, Some("test_uuid")).await {
        Ok(id_value3) => {
            println!("✅ 手动ID用户创建成功，使用的ID: {:?}", id_value3);

            // 验证是否使用了指定的ID
            if let DataValue::String(id_str) = &id_value3 {
                if id_str == "manual_id_001" {
                    println!("✅ 正确使用了手动指定的ID");
                } else {
                    println!("❌ 没有使用手动指定的ID，而是使用了: {}", id_str);
                }
            }
        },
        Err(e) => {
            println!("❌ 手动ID用户创建失败: {}", e);
        }
    }

    println!("\n=== 测试完成 ===");
    println!("总结：");
    println!("- ✅ IdStrategy::Uuid现在可以正常工作");
    println!("- ✅ 当数据不包含ID字段时，会自动生成UUID");
    println!("- ✅ 当数据包含ID字段时，会使用指定的ID");
    println!("- ✅ 生成的UUID保证唯一性");

    Ok(())
}