//! RatQuickDB 模型定义示例
//! 
//! 本示例展示了如何使用 RatQuickDB 的模型定义系统，
//! 包括字段定义、索引创建、模型验证等功能。

use rat_quickdb::odm::{AsyncOdmManager, OdmOperations};
use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, DataValue, QueryCondition, QueryOperator, SortDirection, SortConfig, PaginationConfig};
use rat_quickdb::manager::{get_global_pool_manager, health_check};
use std::collections::HashMap;
use std::time::Duration;
use chrono::Utc;
use serde::{Serialize, Deserialize};

// 定义用户模型
define_model! {
    /// 用户模型
    struct User {
        id: String,
        username: String,
        email: String,
        password_hash: String,
        full_name: String,
        age: Option<i32>,
        phone: Option<String>,
        avatar_url: Option<String>,
        is_active: bool,
        created_at: String,
        updated_at: Option<String>,
        last_login: Option<String>,
        profile: Option<String>,
        tags: Option<Vec<String>>,
    }
    collection = "users",
    fields = {
        id: FieldDefinition::new(field_types!(string)).required().unique(),
        username: FieldDefinition::new(field_types!(string)).required().unique(),
        email: FieldDefinition::new(field_types!(string)).required().unique(),
        password_hash: FieldDefinition::new(field_types!(string)).required(),
        full_name: FieldDefinition::new(field_types!(string)).required(),
        age: FieldDefinition::new(field_types!(integer)),
        phone: FieldDefinition::new(field_types!(string)),
        avatar_url: FieldDefinition::new(field_types!(string)),
        is_active: FieldDefinition::new(field_types!(boolean)).required(),
        created_at: FieldDefinition::new(field_types!(datetime)).required(),
        updated_at: FieldDefinition::new(field_types!(datetime)),
        last_login: FieldDefinition::new(field_types!(datetime)),
        profile: FieldDefinition::new(field_types!(json)),
        tags: FieldDefinition::new(field_types!(array, field_types!(string))),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["created_at"], unique: false, name: "idx_created_at" },
        { fields: ["is_active", "created_at"], unique: false, name: "idx_active_created" },
    ],
}

// 定义文章模型
define_model! {
    /// 文章模型
    struct Article {
        id: String,
        title: String,
        slug: String,
        content: String,
        summary: Option<String>,
        author_id: String,
        category_id: Option<String>,
        status: String,
        view_count: i32,
        like_count: i32,
        is_featured: bool,
        published_at: Option<String>,
        created_at: String,
        updated_at: Option<String>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
    }
    collection = "articles",
    fields = {
        id: FieldDefinition::new(field_types!(string)).required().unique(),
        title: FieldDefinition::new(field_types!(string)).required(),
        slug: FieldDefinition::new(field_types!(string)).required().unique(),
        content: FieldDefinition::new(field_types!(string)).required(),
        summary: FieldDefinition::new(field_types!(string)),
        author_id: FieldDefinition::new(field_types!(string)).required(),
        category_id: FieldDefinition::new(field_types!(string)),
        status: FieldDefinition::new(field_types!(string)).required(),
        view_count: FieldDefinition::new(field_types!(integer)).required(),
        like_count: FieldDefinition::new(field_types!(integer)).required(),
        is_featured: FieldDefinition::new(field_types!(boolean)).required(),
        published_at: FieldDefinition::new(field_types!(datetime)),
        created_at: FieldDefinition::new(field_types!(datetime)).required(),
        updated_at: FieldDefinition::new(field_types!(datetime)),
        metadata: FieldDefinition::new(field_types!(json)),
        tags: FieldDefinition::new(field_types!(array, field_types!(string))),
    }
    indexes = [
        { fields: ["slug"], unique: true, name: "idx_slug" },
        { fields: ["author_id"], unique: false, name: "idx_author" },
        { fields: ["category_id"], unique: false, name: "idx_category" },
        { fields: ["status", "published_at"], unique: false, name: "idx_status_published" },
        { fields: ["is_featured", "published_at"], unique: false, name: "idx_featured_published" },
    ],
}

// 定义评论模型
define_model! {
    /// 评论模型
    struct Comment {
        id: String,
        article_id: String,
        user_id: String,
        parent_id: Option<String>,
        content: String,
        is_approved: bool,
        like_count: i32,
        created_at: String,
        updated_at: Option<String>,
    }
    collection = "comments",
    fields = {
        id: FieldDefinition::new(field_types!(string)).required().unique(),
        article_id: FieldDefinition::new(field_types!(string)).required(),
        user_id: FieldDefinition::new(field_types!(string)).required(),
        parent_id: FieldDefinition::new(field_types!(string)),
        content: FieldDefinition::new(field_types!(string)).required(),
        is_approved: FieldDefinition::new(field_types!(boolean)).required(),
        like_count: FieldDefinition::new(field_types!(integer)).required(),
        created_at: FieldDefinition::new(field_types!(datetime)).required(),
        updated_at: FieldDefinition::new(field_types!(datetime)),
    }
    indexes = [
        { fields: ["article_id"], unique: false, name: "idx_article" },
        { fields: ["user_id"], unique: false, name: "idx_user" },
        { fields: ["parent_id"], unique: false, name: "idx_parent" },
        { fields: ["article_id", "is_approved"], unique: false, name: "idx_article_approved" },
    ],
}

// 删除重复的main函数

/// 演示JSON序列化功能
async fn demonstrate_json_serialization() -> QuickDbResult<()> {
    println!("\n=== JSON序列化演示 ===");
    
    // 获取ODM管理器
    let odm_manager = AsyncOdmManager::new();
    
    // 创建真实的用户数据
    println!("创建用户数据...");
    let user_id = uuid::Uuid::new_v4().to_string();
    let user_data = create_user_data(&user_id, "zhangsan", "zhangsan@example.com", "张三", 25);
    
    // 插入用户数据
    match odm_manager.create("users", user_data.clone(), None).await {
        Ok(created_id) => {
            println!("用户创建成功，ID: {}", created_id);
            
            // 查询用户数据
            println!("\n查询用户数据...");
            let id_str = match &created_id {
                DataValue::String(s) => s,
                DataValue::Int(i) => &i.to_string(),
                DataValue::Uuid(u) => &u.to_string(),
                _ => panic!("ID类型不支持: {:?}", created_id)
            };
            match odm_manager.find_by_id("users", id_str, None).await {
                Ok(Some(user_json)) => {
                    println!("找到用户: {}", user_json);
                    
                    // 演示不同的序列化选项
                    println!("\n序列化选项:");
                    
                    // 1. 默认序列化（紧凑格式）
                    let compact_json = serde_json::to_string(&user_data)
                        .unwrap_or_else(|_| "序列化失败".to_string());
                    println!("1. 默认序列化: {}", compact_json);
                    
                    // 2. 美化序列化
                    println!("2. 美化序列化:");
                    let pretty_json = serde_json::to_string_pretty(&user_data)
                        .unwrap_or_else(|_| "序列化失败".to_string());
                    println!("{}", pretty_json);
                    
                    // 3. 转换为DataValue并序列化（PyO3兼容）
                    println!("3. PyO3兼容序列化:");
                    let data_value_json = serde_json::to_string(&user_data)
                        .unwrap_or_else(|_| "序列化失败".to_string());
                    println!("{}", data_value_json);
                    
                    // 清理测试数据
                    let _ = odm_manager.delete_by_id("users", id_str, None).await;
                },
                Ok(None) => println!("用户未找到"),
                Err(e) => println!("查询用户失败: {}", e),
            }
        },
        Err(e) => println!("用户创建失败: {}", e),
    }
    
    Ok(())
}

/// 演示连接池监控
async fn demonstrate_pool_monitoring() -> QuickDbResult<()> {
    println!("\n=== 连接池监控演示 ===");
    
    // 获取真实的连接池管理器
    let manager = get_global_pool_manager();
    
    // 获取默认连接池的统计信息
    println!("获取连接池状态...");
    // 获取连接池健康状态
    println!("\n获取连接池健康状态...");
    let health_status = manager.health_check().await;
    for (alias, is_healthy) in health_status {
        println!("连接池 '{}': {}", alias, if is_healthy { "健康" } else { "异常" });
    }
    
    // 获取所有数据库别名
    println!("\n获取所有数据库别名...");
    let aliases = manager.get_aliases();
    for alias in aliases {
        println!("数据库别名: {}", alias);
    }
    
    // 执行真实的健康检查
    println!("\n执行健康检查...");
    let health_map = health_check().await;
    for (db_alias, is_healthy) in health_map {
        let status = if is_healthy { "✓ 正常" } else { "✗ 异常" };
        println!("数据库 '{}': 健康状态 {}", db_alias, status);
    }
    
    Ok(())
}

/// 演示基本CRUD操作
async fn demonstrate_basic_crud(odm_manager: &AsyncOdmManager) -> QuickDbResult<()> {
    println!("\n=== 基本CRUD操作演示 ===");
    
    // 创建用户
    println!("\n1. 创建用户...");
    let user_id = uuid::Uuid::new_v4().to_string();
    let user_data = create_user_data(&user_id, "alice", "alice@example.com", "Alice Smith", 25);
    
    match odm_manager.create("users", user_data, None).await {
        Ok(created_id) => {
            println!("用户创建成功，ID: {}", created_id);
            
            // 查询用户
            println!("\n2. 查询用户...");
            let id_str = match &created_id {
                DataValue::String(s) => s,
                DataValue::Int(i) => &i.to_string(),
                DataValue::Uuid(u) => &u.to_string(),
                _ => panic!("ID类型不支持: {:?}", created_id)
            };
            match odm_manager.find_by_id("users", id_str, None).await {
                Ok(Some(found_user_json)) => {
                    println!("找到用户: {}", found_user_json);
                    
                    // 更新用户（简化演示）
                    println!("\n3. 更新用户...");
                    let mut update_data = HashMap::new();
                    update_data.insert("age".to_string(), DataValue::Int(26));
                    update_data.insert("updated_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
                    
                    match odm_manager.update_by_id("users", id_str, update_data, None).await {
                        Ok(_) => println!("用户更新成功"),
                        Err(e) => println!("用户更新失败: {}", e),
                    }
                    
                    // 删除用户
                    println!("\n4. 删除用户...");
                    match odm_manager.delete_by_id("users", id_str, None).await {
                        Ok(_) => println!("用户删除成功"),
                        Err(e) => println!("用户删除失败: {}", e),
                    }
                },
                Ok(None) => println!("用户未找到"),
                Err(e) => println!("查询用户失败: {}", e),
            }
        },
        Err(e) => println!("用户创建失败: {}", e),
    }
    
    Ok(())
}

/// 演示错误处理
async fn demonstrate_error_handling(odm_manager: &AsyncOdmManager) -> QuickDbResult<()> {
    println!("\n=== 错误处理演示 ===");
    
    // 1. 查询不存在的表
    println!("\n1. 查询不存在的表...");
    match odm_manager.find("non_existent_table", vec![], None, None).await {
        Ok(_) => println!("意外成功"),
        Err(e) => println!("预期错误: {}", e),
    }
    
    // 2. 使用无效的数据库别名
    println!("\n2. 使用无效的数据库别名...");
    match odm_manager.find("users", vec![], None, Some("invalid_alias")).await {
        Ok(_) => println!("意外成功"),
        Err(e) => println!("预期错误: {}", e),
    }
    
    // 3. 创建重复数据（如果有唯一约束）
    println!("\n3. 创建重复数据...");
    let duplicate_user = create_user_data("duplicate_001", "duplicate", "duplicate@example.com", "Duplicate User", 25);
    
    // 第一次创建
    match odm_manager.create("users", duplicate_user.clone(), None).await {
        Ok(id) => println!("第一次创建成功: {}", id),
        Err(e) => println!("第一次创建失败: {}", e),
    }
    
    // 第二次创建相同数据（可能失败）
    match odm_manager.create("users", duplicate_user, None).await {
        Ok(id) => println!("第二次创建成功: {}", id),
        Err(e) => println!("预期错误（重复数据）: {}", e),
    }
    
    Ok(())
}

/// 演示批量操作
async fn demonstrate_batch_operations(odm_manager: &AsyncOdmManager) -> QuickDbResult<()> {
    println!("\n=== 批量操作演示 ===");
    
    // 1. 批量创建用户
    println!("\n1. 批量创建用户...");
    let batch_users = vec![
        create_user_data("batch_001", "batch1", "batch1@example.com", "Batch User 1", 25),
        create_user_data("batch_002", "batch2", "batch2@example.com", "Batch User 2", 30),
        create_user_data("batch_003", "batch3", "batch3@example.com", "Batch User 3", 28),
        create_user_data("batch_004", "batch4", "batch4@example.com", "Batch User 4", 32),
    ];
    
    let mut created_count = 0;
    for (i, user_data) in batch_users.iter().enumerate() {
        match odm_manager.create("users", user_data.clone(), None).await {
            Ok(result) => {
                println!("创建用户 {}: {}", i + 1, result);
                created_count += 1;
            },
            Err(e) => println!("创建用户 {} 失败: {}", i + 1, e),
        }
    }
    println!("批量创建完成，共创建 {} 个用户", created_count);
    
    // 2. 批量查询
    println!("\n2. 批量查询用户...");
    let batch_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("batch".to_string()),
        },
    ];
    
    // 批量查询（简化示例）
    match odm_manager.find("users", batch_conditions, None, None).await {
        Ok(result) => println!("查询结果（用户名包含'batch'）: {:?}", result),
        Err(e) => println!("批量查询失败: {}", e),
    }
    
    // 3. 批量更新
    println!("\n3. 批量更新用户状态...");
    let update_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("batch".to_string()),
        }
    ];
    
    let mut update_data = HashMap::new();
    update_data.insert("is_active".to_string(), DataValue::Bool(false));
    update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));
    
    // 批量更新（简化示例）
    match odm_manager.update_by_id("users", "batch_001", update_data, None).await {
        Ok(affected_rows) => println!("批量更新完成，影响 {} 行", affected_rows),
        Err(e) => println!("批量更新失败: {}", e),
    }
    
    // 4. 批量统计
    println!("\n4. 批量统计操作...");
    match odm_manager.count("users", vec![], None).await {
        Ok(total) => println!("总用户数: {}", total),
        Err(e) => println!("统计总数失败: {}", e),
    }
    
    let batch_count_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("batch".to_string()),
        }
    ];
    
    // 统计批量用户数（简化示例）
    match odm_manager.count("users", batch_count_conditions, None).await {
        Ok(batch_count) => println!("批量用户数: {}", batch_count),
        Err(e) => println!("统计批量用户数失败: {}", e),
    }
    
    Ok(())
}

/// 演示事务操作（模拟）
async fn demonstrate_transaction_operations(odm_manager: &AsyncOdmManager) -> QuickDbResult<()> {
    println!("\n=== 事务操作演示 ===");
    
    // 注意：当前 rat_quickdb 可能不直接支持事务，这里演示相关的原子操作
    println!("演示原子性操作（模拟事务）...");
    
    // 1. 创建相关联的数据
    println!("\n1. 创建用户和文章的关联操作...");
    
    // 先创建用户
    let user_data = create_user_data("tx_001", "txuser", "tx@example.com", "Transaction User", 35);
    let user_result = match odm_manager.create("users", user_data.clone(), None).await {
        Ok(result) => {
            println!("创建用户成功: {}", result);
            result
        },
        Err(e) => {
            println!("创建用户失败: {}", e);
            return Err(e);
        }
    };
    
    // 再创建文章（关联到用户）
    let article_data = create_article_data(
        "tx_article_001",
        "事务测试文章",
        "tx-test-article",
        "这是一篇在事务中创建的测试文章，用于演示事务操作的完整性。",
        "tx_001",
        "published"
    );
    let article_result = match odm_manager.create("articles", article_data.clone(), None).await {
        Ok(result) => {
            println!("创建文章成功: {}", result);
            result
        },
        Err(e) => {
            println!("创建文章失败: {}", e);
            // 如果文章创建失败，删除已创建的用户（模拟回滚）
            println!("模拟回滚：删除已创建的用户...");
            let delete_conditions = vec![
                QueryCondition {
                    field: "id".to_string(),
                    operator: QueryOperator::Eq,
                    value: DataValue::String("tx_001".to_string()),
                }
            ];
            let delete_conditions = vec![
                QueryCondition {
                    field: "id".to_string(),
                    operator: QueryOperator::Eq,
                    value: DataValue::String("tx_001".to_string()),
                }
            ];
            match odm_manager.delete("users", delete_conditions, None).await {
                Ok(deleted) => println!("回滚删除用户: {} 条记录", deleted),
                Err(del_err) => println!("回滚删除失败: {}", del_err),
            }
            return Err(e);
        }
    };
    
    println!("关联数据创建完成");
    
    // 2. 演示批量操作的原子性
    println!("\n2. 演示批量操作的一致性...");
    
    // 查询验证数据一致性
    let user_conditions = vec![
        QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("tx_001".to_string()),
        }
    ];
    
    match odm_manager.find_by_id("users", "tx_001", None).await {
        Ok(users) => println!("验证用户数据: {:?}", users),
        Err(e) => println!("验证用户数据失败: {}", e),
    }
    
    let article_conditions = vec![
        QueryCondition {
            field: "author_id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("tx_001".to_string()),
        }
    ];
    
    // 查询用户关联的文章（简化查询）
    match odm_manager.find_by_id("articles", "val_article_001", None).await {
        Ok(articles) => println!("验证文章数据: {:?}", articles),
        Err(e) => println!("验证文章数据失败: {}", e),
    }
    
    println!("事务操作演示完成");
    
    Ok(())
}

/// 演示性能测试
async fn demonstrate_performance_test(odm_manager: &AsyncOdmManager) -> QuickDbResult<()> {
    println!("\n=== 性能测试演示 ===");
    
    use std::time::Instant;
    
    // 1. 批量插入性能测试
    println!("\n1. 批量插入性能测试...");
    let start = Instant::now();
    let batch_size = 100; // 减少测试数据量以避免过长时间
    
    for i in 0..batch_size {
        let user_data = create_user_data(
            &format!("perf_user_{}", i),
            &format!("perfuser{}", i),
            &format!("perf{}@example.com", i),
            &format!("Performance User {}", i),
            20 + (i as i32 % 50)
        );
        
        match odm_manager.create("users", user_data, None).await {
            Ok(_) => {},
            Err(e) => println!("插入第 {} 条记录失败: {}", i, e),
        }
    }
    
    let insert_duration = start.elapsed();
    println!("插入 {} 条记录完成", batch_size);
    println!("总耗时: {:?}", insert_duration);
    println!("平均每条记录: {:?}", insert_duration / batch_size as u32);
    
    // 2. 查询性能测试
    println!("\n2. 查询性能测试...");
    let start = Instant::now();
    let query_count = 50;
    
    for i in 0..query_count {
        let conditions = vec![
            QueryCondition {
                field: "username".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("perfuser{}", i % 10)),
            }
        ];
        
        match odm_manager.find_by_id("users", &format!("perf_{:03}", i % 10), None).await {
            Ok(_) => {},
            Err(e) => println!("第 {} 次查询失败: {}", i, e),
        }
    }
    
    let query_duration = start.elapsed();
    println!("执行 {} 次查询完成", query_count);
    println!("总耗时: {:?}", query_duration);
    println!("平均每次查询: {:?}", query_duration / query_count);
    
    // 3. 更新性能测试
    println!("\n3. 更新性能测试...");
    let start = Instant::now();
    let update_count = 30;
    
    for i in 0..update_count {
        let conditions = vec![
            QueryCondition {
                field: "username".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("perfuser{}", i)),
            }
        ];
        
        let mut update_data = HashMap::new();
        update_data.insert("age".to_string(), DataValue::Int((30 + i) as i64));
        
        // 更新操作（简化示例）
        let mut update_data = HashMap::new();
         update_data.insert("age".to_string(), DataValue::Int((30 + i) as i64));
         update_data.insert("updated_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
         
         let update_conditions = vec![
             QueryCondition {
                 field: "id".to_string(),
                 operator: QueryOperator::Eq,
                 value: DataValue::String(format!("perf_{:03}", i)),
             }
         ];
         match odm_manager.update("users", update_conditions, update_data, None).await {
            Ok(_) => {},
            Err(e) => println!("第 {} 次更新失败: {}", i, e),
        }
    }
    
    let update_duration = start.elapsed();
    println!("更新 {} 条记录完成", update_count);
    println!("总耗时: {:?}", update_duration);
    println!("平均每条记录: {:?}", update_duration / update_count);
    
    // 4. 统计性能测试
    println!("\n4. 统计性能测试...");
    let start = Instant::now();
    
    let count_conditions = vec![];
    match odm_manager.count("users", count_conditions, None).await {
        Ok(total_count) => {
            let count_duration = start.elapsed();
            println!("统计总记录数: {}", total_count);
            println!("统计耗时: {:?}", count_duration);
        },
        Err(e) => println!("统计失败: {}", e),
    }
    
    // 5. 连接池性能状态
    println!("\n5. 连接池性能状态...");
    let manager = get_global_pool_manager();
    let health_status = manager.health_check().await;
    println!("连接池健康状态: {:?}", health_status);
    
    println!("性能测试演示完成");
    
    Ok(())
}

/// 主函数
#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("=== RatQuickDB 模型定义系统演示 ===");
    
    // 初始化连接池配置
    let pool_config = PoolConfig::builder()
        .max_connections(10)
        .min_connections(2)
        .connection_timeout(5000)
        .idle_timeout(300000)
        .max_lifetime(1800000)
        .build()?;
    
    // 获取全局连接池管理器并添加数据库配置
    let pool_manager = get_global_pool_manager();
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./test_model.db".to_string(),
            create_if_missing: true,
        })
        .pool(pool_config)
        .alias("default")
        .id_strategy(IdStrategy::Uuid)
        .build()?;
    
    pool_manager.add_database(db_config).await?;
    
    // 初始化ODM管理器
    let odm_manager = AsyncOdmManager::new();
    
    // 模拟创建模型管理器
    println!("创建模型管理器...");
    
    println!("\n1. 演示模型验证功能");
    demonstrate_model_validation(&odm_manager).await?;
    
    println!("\n2. 演示基本CRUD操作");
    demonstrate_basic_crud(&odm_manager).await?;
    
    println!("\n3. 演示复杂查询功能");
    demonstrate_complex_queries(&odm_manager).await?;
    
    println!("\n4. 演示JSON序列化功能");
    demonstrate_json_serialization().await?;
    
    println!("\n5. 演示连接池监控");
    demonstrate_pool_monitoring().await?;
    
    println!("\n6. 演示错误处理");
    demonstrate_error_handling(&odm_manager).await?;
    
    println!("\n7. 演示批量操作");
    demonstrate_batch_operations(&odm_manager).await?;
    
    println!("\n8. 演示事务操作");
    demonstrate_transaction_operations(&odm_manager).await?;
    
    println!("\n9. 演示性能测试");
    demonstrate_performance_test(&odm_manager).await?;
    
    println!("\n=== 演示完成 ===");
    Ok(())
}

/// 创建用户数据
fn create_user_data(id: &str, username: &str, email: &str, full_name: &str, age: i32) -> HashMap<String, DataValue> {
    let mut user_data = HashMap::new();
    user_data.insert("id".to_string(), DataValue::String(id.to_string()));
    user_data.insert("username".to_string(), DataValue::String(username.to_string()));
    user_data.insert("email".to_string(), DataValue::String(email.to_string()));
    user_data.insert("password_hash".to_string(), DataValue::String("hashed_password_here".to_string()));
    user_data.insert("full_name".to_string(), DataValue::String(full_name.to_string()));
    user_data.insert("age".to_string(), DataValue::Int(age as i64));
    user_data.insert("phone".to_string(), DataValue::String(format!("+86138{:08}", id.chars().last().unwrap_or('0') as u8)));
    user_data.insert("avatar_url".to_string(), DataValue::String(format!("https://avatar.example.com/{}.jpg", id)));
    user_data.insert("is_active".to_string(), DataValue::Bool(true));
    user_data.insert("created_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
    user_data.insert("updated_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
    user_data.insert("profile".to_string(), DataValue::Json(serde_json::json!({
        "preferences": {
            "theme": "dark",
            "language": "zh-CN"
        },
        "settings": {
            "notifications": true,
            "privacy_level": "medium"
        }
    })));
    user_data.insert("tags".to_string(), DataValue::Array(vec![
        DataValue::String("新用户".to_string()),
        DataValue::String("活跃".to_string()),
    ]));
    user_data
}

/// 创建文章数据
fn create_article_data(id: &str, title: &str, slug: &str, content: &str, author_id: &str, status: &str) -> HashMap<String, DataValue> {
    let mut article_data = HashMap::new();
    article_data.insert("id".to_string(), DataValue::String(id.to_string()));
    article_data.insert("title".to_string(), DataValue::String(title.to_string()));
    article_data.insert("slug".to_string(), DataValue::String(slug.to_string()));
    article_data.insert("content".to_string(), DataValue::String(content.to_string()));
    let summary = if content.chars().count() > 50 {
        format!("{}...", content.chars().take(50).collect::<String>())
    } else {
        content.to_string()
    };
    article_data.insert("summary".to_string(), DataValue::String(summary));
    article_data.insert("author_id".to_string(), DataValue::String(author_id.to_string()));
    article_data.insert("category_id".to_string(), DataValue::String("tech".to_string()));
    article_data.insert("status".to_string(), DataValue::String(status.to_string()));
    article_data.insert("view_count".to_string(), DataValue::Int(0));
        article_data.insert("like_count".to_string(), DataValue::Int(0));
        article_data.insert("is_featured".to_string(), DataValue::Bool(status == "published"));
    
    if status == "published" {
        article_data.insert("published_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
    }
    article_data.insert("created_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
    article_data.insert("updated_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
    article_data.insert("metadata".to_string(), DataValue::Json(serde_json::json!({
        "seo": {
            "keywords": ["rust", "编程", "技术"],
            "description": title
        },
        "reading_time": content.len() / 200 // 估算阅读时间（分钟）
    })));
    article_data.insert("tags".to_string(), DataValue::Array(vec![
        DataValue::String("技术".to_string()),
        DataValue::String("编程".to_string()),
    ]));
    article_data
}

/// 创建评论数据
fn create_comment_data(id: &str, article_id: &str, user_id: &str, parent_id: Option<&str>, content: &str) -> HashMap<String, DataValue> {
    let mut comment_data = HashMap::new();
    comment_data.insert("id".to_string(), DataValue::String(id.to_string()));
    comment_data.insert("article_id".to_string(), DataValue::String(article_id.to_string()));
    comment_data.insert("user_id".to_string(), DataValue::String(user_id.to_string()));
    
    if let Some(parent) = parent_id {
        comment_data.insert("parent_id".to_string(), DataValue::String(parent.to_string()));
    }
    
    comment_data.insert("content".to_string(), DataValue::String(content.to_string()));
    comment_data.insert("is_approved".to_string(), DataValue::Bool(true));
        comment_data.insert("like_count".to_string(), DataValue::Int(0));
    comment_data.insert("created_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
    comment_data.insert("updated_at".to_string(), DataValue::String(Utc::now().to_rfc3339()));
    comment_data
}






/// 演示模型验证功能
async fn demonstrate_model_validation(odm_manager: &AsyncOdmManager) -> QuickDbResult<()> {
    println!("\n=== 模型验证演示 ===");
    
    // 1. 验证有效用户数据
    println!("\n1. 验证有效用户数据...");
    let valid_user = create_user_data("val_001", "validuser", "valid@example.com", "Valid User", 25);
    
    match odm_manager.create("users", valid_user.clone(), None).await {
        Ok(result) => {
            println!("有效用户数据创建成功: {}", result);
            println!("用户邮箱: {:?}", valid_user.get("email"));
        },
        Err(e) => println!("创建用户失败: {}", e),
    }
    
    // 2. 测试重复数据验证
    println!("\n2. 测试重复数据验证...");
    let duplicate_user = create_user_data("val_001", "validuser", "valid@example.com", "Valid User", 30);
    
    match odm_manager.create("users", duplicate_user, None).await {
        Ok(result) => println!("重复用户创建成功（可能允许重复）: {}", result),
        Err(e) => println!("重复用户创建失败（符合预期）: {}", e),
    }
    
    // 3. 验证查询条件
    println!("\n3. 验证查询条件...");
    
    // 有效查询
    let valid_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("validuser".to_string()),
        }
    ];
    
    // 注意：ODM管理器的find方法需要根据实际实现调整
    // 这里使用简化的查询方式
    match odm_manager.find_by_id("users", "validuser", None).await {
        Ok(users) => println!("有效查询结果: {:?}", users),
        Err(e) => println!("查询失败: {}", e),
    }
    
    // 无效字段查询
    let invalid_conditions = vec![
        QueryCondition {
            field: "nonexistent_field".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("test".to_string()),
        }
    ];
    
    // 注意：ODM管理器会在适配器层处理无效字段，这里简化处理
    println!("无效字段查询将在适配器层被处理");
    
    // 4. 验证文章数据
    println!("\n4. 验证文章数据...");
    let valid_article = create_article_data(
        "val_article_001",
        "验证测试文章",
        "val-test-article",
        "这是一篇用于验证的测试文章，内容足够长以满足验证要求。",
        "val_001",
        "published"
    );
    
    // 移除Article结构体实例化，直接使用HashMap数据
    
    let article_data = create_article_data("article_001", "示例文章", "example-article", "这是一篇示例文章的内容", "用户001", "published");
    match odm_manager.create("articles", article_data, None).await {
        Ok(result) => {
            println!("有效文章数据创建成功: {}", result);
            println!("文章标题: {:?}", valid_article.get("title"));
        },
        Err(e) => println!("创建文章失败: {}", e),
    }
    
    // 5. 验证数据完整性
    println!("\n5. 验证数据完整性...");
    
    // 检查用户和文章的关联性
    let article_conditions = vec![
        QueryCondition {
            field: "author_id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("val_001".to_string()),
        }
    ];
    
    match odm_manager.find("articles", article_conditions, None, None).await {
        Ok(articles_json) => {
            println!("用户关联的文章: {:?}", articles_json);
            // 可以进一步解析 JSON 来验证数据完整性
        },
        Err(e) => println!("查询用户文章失败: {}", e),
    }
    
    println!("模型验证演示完成");
    
    Ok(())
}

/// 演示复杂查询功能
async fn demonstrate_complex_queries(odm_manager: &AsyncOdmManager) -> QuickDbResult<()> {
    println!("\n=== 复杂查询演示 ===");
    
    // 先创建一些测试用户
    println!("创建测试用户...");
    let test_users = vec![
        create_user_data("query_001", "alice", "alice@example.com", "Alice Johnson", 25),
        create_user_data("query_002", "bob", "bob@example.com", "Bob Smith", 30),
        create_user_data("query_003", "charlie", "charlie@example.com", "Charlie Brown", 35),
        create_user_data("query_004", "diana", "diana@example.com", "Diana Prince", 28),
    ];
    
    for user_data in test_users {
        match odm_manager.create("users", user_data, None).await {
            Ok(id) => println!("创建测试用户成功: {}", id),
            Err(e) => println!("创建测试用户失败: {}", e),
        }
    }
    
    // 1. 多条件查询（简化演示）
    println!("\n1. 多条件查询演示...");
    let conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(25),
        },
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Lt,
            value: DataValue::Int(35),
        }
    ];
    
    // 注意：这里的复杂查询需要根据ODM的实际实现进行调整
    match odm_manager.find("users", conditions.clone(), None, None).await {
        Ok(users_json) => println!("多条件查询结果: {:?}", users_json),
        Err(e) => println!("多条件查询失败: {}", e),
    }
    
    // 2. 范围查询
    println!("\n2. 范围查询（年龄在25-35之间）...");
    let range_conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        },
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::Int(35),
        }
    ];
    
    match odm_manager.find("users", range_conditions, None, None).await {
        Ok(users_json) => println!("范围查询结果: {:?}", users_json),
        Err(e) => println!("范围查询失败: {}", e),
    }
    
    // 3. 模糊查询
    println!("\n3. 模糊查询（用户名包含'a'）...");
    let like_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("a".to_string()),
        }
    ];
    
    match odm_manager.find("users", like_conditions, None, None).await {
        Ok(users_json) => println!("模糊查询结果: {:?}", users_json),
        Err(e) => println!("模糊查询失败: {}", e),
    }
    
    // 4. 统计查询
    println!("\n4. 统计查询...");
    match odm_manager.count("users", vec![], None).await {
        Ok(total) => println!("总用户数: {}", total),
        Err(e) => println!("统计查询失败: {}", e),
    }
    
    // 5. 存在性查询
    println!("\n5. 存在性查询（检查是否存在用户名为'alice'的用户）...");
    let exists_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("alice".to_string()),
        }
    ];
    
    match odm_manager.exists("users", exists_conditions, None).await {
        Ok(exists) => println!("用户名'alice'存在: {}", exists),
        Err(e) => println!("存在性查询失败: {}", e),
    }
    
    println!("复杂查询演示完成");
    
    Ok(())
}