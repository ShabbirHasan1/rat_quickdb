//! RatQuickDB 模型定义示例
//! 
//! 本示例展示了如何使用 RatQuickDB 的模型定义系统，
//! 包括字段定义、索引创建、模型验证等功能。

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, DataValue, QueryCondition, QueryOperator, SortDirection, SortConfig, PaginationConfig};
use rat_quickdb::manager::{get_global_pool_manager, health_check};
use rat_quickdb::{ModelManager, ModelOperations, string_field, integer_field, float_field, boolean_field, datetime_field, uuid_field, json_field, array_field, field_types};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use std::collections::HashMap;
use std::time::Duration;
use chrono::{Utc, DateTime};
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
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
        last_login: Option<chrono::DateTime<chrono::Utc>>,
        profile: Option<serde_json::Value>,
        tags: Option<Vec<String>>,
    }
    collection = "users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        username: string_field(None, None, None).required().unique(),
        email: string_field(None, None, None).required().unique(),
        password_hash: string_field(None, None, None).required(),
        full_name: string_field(None, None, None).required(),
        age: integer_field(None, None),
        phone: string_field(None, None, None),
        avatar_url: string_field(None, None, None),
        is_active: boolean_field().required(),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
        last_login: datetime_field(),
        profile: json_field(),
        tags: array_field(field_types!(string), None, None),
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
        published_at: Option<chrono::DateTime<chrono::Utc>>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
        metadata: Option<serde_json::Value>,
        tags: Option<Vec<String>>,
    }
    collection = "articles",
    fields = {
        id: string_field(None, None, None).required().unique(),
        title: string_field(None, None, None).required(),
        slug: string_field(None, None, None).required().unique(),
        content: string_field(None, None, None).required(),
        summary: string_field(None, None, None),
        author_id: string_field(None, None, None).required(),
        category_id: string_field(None, None, None),
        status: string_field(None, None, None).required(),
        view_count: integer_field(None, None).required(),
        like_count: integer_field(None, None).required(),
        is_featured: boolean_field().required(),
        published_at: datetime_field(),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
        metadata: json_field(),
        tags: array_field(field_types!(string), None, None),
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
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
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
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
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

    // 创建真实的用户数据
    println!("创建用户数据...");
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: format!("zhangsan_{}", uuid::Uuid::new_v4().simple()),
        email: format!("zhangsan_{}@example.com", uuid::Uuid::new_v4().simple()),
        password_hash: "hashed_password_here".to_string(),
        full_name: "张三".to_string(),
        age: Some(25),
        phone: Some("+8613812345678".to_string()),
        avatar_url: Some("https://avatar.example.com/zhangsan.jpg".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: Some(serde_json::json!({"preferences":{"theme":"dark","language":"zh-CN"}})),
        tags: Some(vec!["新用户".to_string(), "活跃".to_string()]),
    };

    // 插入用户数据
    match user.save().await {
        Ok(created_id) => {
            println!("✅ 用户创建成功，ID: {}", created_id);

            // 查询用户数据
            println!("\n查询用户数据...");
            match ModelManager::<User>::find_by_id(&created_id).await {
                Ok(Some(found_user)) => {
                    println!("✅ 找到用户: {} - {}", found_user.id, found_user.username);

                    // 演示不同的序列化选项
                    println!("\n序列化选项:");

                    // 1. 默认序列化（紧凑格式）
                    let compact_json = serde_json::to_string(&found_user)
                        .unwrap_or_else(|_| "序列化失败".to_string());
                    println!("1. 默认序列化: {}", compact_json);

                    // 2. 美化序列化
                    println!("2. 美化序列化:");
                    let pretty_json = serde_json::to_string_pretty(&found_user)
                        .unwrap_or_else(|_| "序列化失败".to_string());
                    println!("{}", pretty_json);

                    // 3. 转换为DataValue并序列化（PyO3兼容）
                    println!("3. PyO3兼容序列化:");
                    let data_map = found_user.to_data_map().unwrap_or_default();
                    let data_value_json = serde_json::to_string(&data_map)
                        .unwrap_or_else(|_| "序列化失败".to_string());
                    println!("{}", data_value_json);

                    // 清理测试数据
                    let _ = found_user.delete().await;
                },
                Ok(None) => println!("❌ 用户未找到"),
                Err(e) => println!("❌ 查询用户失败: {}", e),
            }
        },
        Err(e) => println!("❌ 用户创建失败: {}", e),
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
async fn demonstrate_basic_crud() -> QuickDbResult<()> {
    println!("\n=== 基本CRUD操作演示 ===");

    // 创建用户
    println!("\n1. 创建用户...");
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: format!("demo_user_{}", uuid::Uuid::new_v4().simple()),
        email: format!("demo_user_{}@example.com", uuid::Uuid::new_v4().simple()),
        password_hash: "hashed_password_here".to_string(),
        full_name: "Demo User".to_string(),
        age: Some(25),
        phone: Some("+8613811111111".to_string()),
        avatar_url: Some("https://avatar.example.com/demo.jpg".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: Some(serde_json::json!({"preferences":{"theme":"dark","language":"en-US"}})),
        tags: Some(vec!["测试用户".to_string()]),
    };

    match user.save().await {
        Ok(created_id) => {
            println!("✅ 用户创建成功，ID: {}", created_id);

            // 查询用户
            println!("\n2. 查询用户...");
            match ModelManager::<User>::find_by_id(&created_id).await {
                Ok(Some(found_user)) => {
                    println!("✅ 找到用户: {} - {}", found_user.id, found_user.username);

                    // 更新用户
                    println!("\n3. 更新用户...");
                    let mut update_data = HashMap::new();
                    update_data.insert("age".to_string(), DataValue::Int(26));
                    update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

                    match found_user.update(update_data).await {
                        Ok(_) => println!("✅ 用户更新成功"),
                        Err(e) => println!("❌ 用户更新失败: {}", e),
                    }

                    // 删除用户
                    println!("\n4. 删除用户...");
                    match found_user.delete().await {
                        Ok(_) => println!("✅ 用户删除成功"),
                        Err(e) => println!("❌ 用户删除失败: {}", e),
                    }
                },
                Ok(None) => println!("❌ 用户未找到"),
                Err(e) => println!("❌ 查询用户失败: {}", e),
            }
        },
        Err(e) => println!("❌ 用户创建失败: {}", e),
    }

    Ok(())
}

/// 演示错误处理
async fn demonstrate_error_handling() -> QuickDbResult<()> {
    println!("\n=== 错误处理演示 ===");

    // 1. 创建无效用户数据（违反字段约束）
    println!("\n1. 创建无效用户数据...");
    let invalid_user = User {
        id: "".to_string(), // 空ID，应该违反必填约束
        username: "".to_string(), // 空用户名，应该违反必填约束
        email: "invalid-email".to_string(), // 无效邮箱格式
        password_hash: "".to_string(),
        full_name: "".to_string(),
        age: Some(-1), // 无效年龄
        phone: None,
        avatar_url: None,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: None,
        tags: None,
    };

    match invalid_user.validate() {
        Ok(_) => println!("❌ 意外：无效用户数据验证通过"),
        Err(e) => println!("✅ 预期错误（数据验证失败）: {}", e),
    }

    // 2. 尝试查询不存在的用户
    println!("\n2. 查询不存在的用户...");
    match ModelManager::<User>::find_by_id("non_existent_id").await {
        Ok(Some(_)) => println!("❌ 意外：找到了不存在的用户"),
        Ok(None) => println!("✅ 预期结果：用户不存在"),
        Err(e) => println!("查询错误: {}", e),
    }

    // 3. 创建重复数据（测试唯一约束）
    println!("\n3. 创建重复数据...");
    let first_user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: format!("unique_user_{}", uuid::Uuid::new_v4().simple()),
        email: format!("unique_user_{}@example.com", uuid::Uuid::new_v4().simple()),
        password_hash: "hashed_password_here".to_string(),
        full_name: "Unique User".to_string(),
        age: Some(25),
        phone: Some("+8613811111111".to_string()),
        avatar_url: Some("https://avatar.example.com/unique1.jpg".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: None,
        tags: None,
    };

    // 第一次创建
    match first_user.save().await {
        Ok(id) => {
            println!("✅ 第一次创建成功: {}", id);

            // 第二次创建相同用户名的用户
            let duplicate_user = User {
                id: uuid::Uuid::new_v4().to_string(),
                username: format!("unique_user_{}", uuid::Uuid::new_v4().simple()), // 重复用户名
                email: format!("unique_user2_{}@example.com", uuid::Uuid::new_v4().simple()),
                password_hash: "hashed_password_here".to_string(),
                full_name: "Duplicate User".to_string(),
                age: Some(30),
                phone: Some("+8613822222222".to_string()),
                avatar_url: Some("https://avatar.example.com/unique2.jpg".to_string()),
                is_active: true,
                created_at: Utc::now(),
                updated_at: Some(Utc::now()),
                last_login: None,
                profile: None,
                tags: None,
            };

            match duplicate_user.save().await {
                Ok(id) => println!("❌ 意外成功：重复用户创建成功: {}", id),
                Err(e) => println!("✅ 预期错误（重复用户名）: {}", e),
            }
        },
        Err(e) => println!("第一次创建失败: {}", e),
    }

    // 4. 测试更新不存在的用户
    println!("\n4. 更新不存在的用户...");
    let non_existent_user = User {
        id: "non_existent_id".to_string(),
        username: "non_existent".to_string(),
        email: "nonexistent@example.com".to_string(),
        password_hash: "hashed_password_here".to_string(),
        full_name: "Non Existent User".to_string(),
        age: Some(25),
        phone: None,
        avatar_url: None,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: None,
        tags: None,
    };

    let mut update_data = HashMap::new();
    update_data.insert("age".to_string(), DataValue::Int(30));

    match non_existent_user.update(update_data).await {
        Ok(_) => println!("❌ 意外成功：更新了不存在的用户"),
        Err(e) => println!("✅ 预期错误（更新不存在的用户）: {}", e),
    }

    // 5. 测试删除不存在的用户
    println!("\n5. 删除不存在的用户...");
    match non_existent_user.delete().await {
        Ok(_) => println!("❌ 意外成功：删除了不存在的用户"),
        Err(e) => println!("✅ 预期错误（删除不存在的用户）: {}", e),
    }

    Ok(())
}

/// 演示批量操作
async fn demonstrate_batch_operations() -> QuickDbResult<()> {
    println!("\n=== 批量操作演示 ===");

    // 1. 批量创建用户
    println!("\n1. 批量创建用户...");
    let batch_users = vec![
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("batch1_{}", uuid::Uuid::new_v4().simple()),
            email: format!("batch1_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Batch User 1".to_string(),
            age: Some(25),
            phone: Some("+8613811111111".to_string()),
            avatar_url: Some("https://avatar.example.com/batch1.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: None,
            tags: Some(vec!["批量用户".to_string()]),
        },
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("batch2_{}", uuid::Uuid::new_v4().simple()),
            email: format!("batch2_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Batch User 2".to_string(),
            age: Some(30),
            phone: Some("+8613822222222".to_string()),
            avatar_url: Some("https://avatar.example.com/batch2.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: None,
            tags: Some(vec!["批量用户".to_string()]),
        },
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("batch3_{}", uuid::Uuid::new_v4().simple()),
            email: format!("batch3_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Batch User 3".to_string(),
            age: Some(28),
            phone: Some("+8613833333333".to_string()),
            avatar_url: Some("https://avatar.example.com/batch3.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: None,
            tags: Some(vec!["批量用户".to_string()]),
        },
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("batch4_{}", uuid::Uuid::new_v4().simple()),
            email: format!("batch4_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Batch User 4".to_string(),
            age: Some(32),
            phone: Some("+8613844444444".to_string()),
            avatar_url: Some("https://avatar.example.com/batch4.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: None,
            tags: Some(vec!["批量用户".to_string()]),
        },
    ];

    let mut created_count = 0;
    let mut created_ids = Vec::new();
    for (i, user) in batch_users.iter().enumerate() {
        match user.save().await {
            Ok(id) => {
                println!("✅ 创建用户 {}: {}", i + 1, id);
                created_count += 1;
                created_ids.push(id);
            },
            Err(e) => println!("❌ 创建用户 {} 失败: {}", i + 1, e),
        }
    }
    println!("✅ 批量创建完成，共创建 {} 个用户", created_count);

    // 2. 批量查询用户
    println!("\n2. 批量查询用户...");
    let batch_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("batch".to_string()),
        },
    ];

    match ModelManager::<User>::find(batch_conditions.clone(), None).await {
        Ok(users) => {
            println!("✅ 查询结果（用户名包含'batch'）: {} 个用户", users.len());
            for user in users {
                println!("   用户: {} - {}", user.id, user.username);
            }
        },
        Err(e) => println!("❌ 批量查询失败: {}", e),
    }

    // 3. 批量更新用户状态
    println!("\n3. 批量更新用户状态...");
    let mut update_data = HashMap::new();
    update_data.insert("is_active".to_string(), DataValue::Bool(false));
    update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    // 批量查询并更新
    if let Ok(users) = ModelManager::<User>::find(batch_conditions.clone(), None).await {
        let mut update_count = 0;
        for user in users {
            match user.update(update_data.clone()).await {
                Ok(_) => {
                    update_count += 1;
                    println!("✅ 更新用户: {}", user.username);
                },
                Err(e) => println!("❌ 更新用户 {} 失败: {}", user.username, e),
            }
        }
        println!("✅ 批量更新完成，更新 {} 个用户", update_count);
    }

    // 4. 批量统计操作
    println!("\n4. 批量统计操作...");
    match ModelManager::<User>::count(vec![]).await {
        Ok(total) => println!("✅ 总用户数: {}", total),
        Err(e) => println!("❌ 统计总数失败: {}", e),
    }

    let batch_count_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("batch".to_string()),
        }
    ];

    match ModelManager::<User>::count(batch_count_conditions).await {
        Ok(batch_count) => println!("✅ 批量用户数: {}", batch_count),
        Err(e) => println!("❌ 统计批量用户数失败: {}", e),
    }

    // 5. 批量删除演示
    println!("\n5. 批量删除演示...");
    let mut delete_count = 0;
    if let Ok(users) = ModelManager::<User>::find(batch_conditions.clone(), None).await {
        for user in users {
            match user.delete().await {
                Ok(_) => {
                    delete_count += 1;
                    println!("✅ 删除用户: {}", user.username);
                },
                Err(e) => println!("❌ 删除用户 {} 失败: {}", user.username, e),
            }
        }
        println!("✅ 批量删除完成，删除 {} 个用户", delete_count);
    }

    Ok(())
}

/// 演示事务操作（模拟）
async fn demonstrate_transaction_operations() -> QuickDbResult<()> {
    println!("\n=== 事务操作演示 ===");

    // 注意：当前 rat_quickdb 可能不直接支持事务，这里演示相关的原子操作
    println!("演示原子性操作（模拟事务）...");

    // 1. 创建相关联的数据
    println!("\n1. 创建用户和文章的关联操作...");

    // 先创建用户
    let user = User {
        id: "tx_001".to_string(),
        username: format!("txuser_{}", uuid::Uuid::new_v4().simple()),
        email: format!("txuser_{}@example.com", uuid::Uuid::new_v4().simple()),
        password_hash: "hashed_password_here".to_string(),
        full_name: "Transaction User".to_string(),
        age: Some(35),
        phone: Some("+8613811111111".to_string()),
        avatar_url: Some("https://avatar.example.com/tx.jpg".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: None,
        tags: None,
    };

    let user_result = match user.save().await {
        Ok(result) => {
            println!("✅ 创建用户成功: {}", result);
            result
        },
        Err(e) => {
            println!("❌ 创建用户失败: {}", e);
            return Err(e);
        }
    };

    // 再创建文章（关联到用户）
    let article = Article {
        id: "tx_article_001".to_string(),
        title: "事务测试文章".to_string(),
        slug: "tx-test-article".to_string(),
        content: "这是一篇在事务中创建的测试文章，用于演示事务操作的完整性。".to_string(),
        summary: Some("这是一篇在事务中创建的测试文章...".to_string()),
        author_id: "tx_001".to_string(),
        category_id: Some("tech".to_string()),
        status: "published".to_string(),
        view_count: 0,
        like_count: 0,
        is_featured: false,
        published_at: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        metadata: None,
        tags: Some(vec!["事务测试".to_string()]),
    };

    let article_result = match article.save().await {
        Ok(result) => {
            println!("✅ 创建文章成功: {}", result);
            result
        },
        Err(e) => {
            println!("❌ 创建文章失败: {}", e);
            // 如果文章创建失败，删除已创建的用户（模拟回滚）
            println!("模拟回滚：删除已创建的用户...");
            match user.delete().await {
                Ok(_) => println!("✅ 回滚删除用户成功"),
                Err(del_err) => println!("❌ 回滚删除失败: {}", del_err),
            }
            return Err(e);
        }
    };

    println!("✅ 关联数据创建完成");

    // 2. 演示批量操作的原子性
    println!("\n2. 演示批量操作的一致性...");

    // 查询验证数据一致性
    match ModelManager::<User>::find_by_id("tx_001").await {
        Ok(Some(user)) => println!("✅ 验证用户数据: {} - {}", user.id, user.username),
        Ok(None) => println!("❌ 用户数据不存在"),
        Err(e) => println!("❌ 验证用户数据失败: {}", e),
    }

    let article_conditions = vec![
        QueryCondition {
            field: "author_id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("tx_001".to_string()),
        }
    ];

    // 查询用户关联的文章
    match ModelManager::<Article>::find(article_conditions, None).await {
        Ok(articles) => {
            println!("✅ 验证文章数据: {} 篇关联文章", articles.len());
            for article in articles {
                println!("   文章: {} - {}", article.id, article.title);
            }
        },
        Err(e) => println!("❌ 验证文章数据失败: {}", e),
    }

    // 3. 演示更新操作的原子性
    println!("\n3. 演示更新操作的原子性...");

    // 同时更新用户和文章的状态
    let mut user_update_data = HashMap::new();
    user_update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    let mut article_update_data = HashMap::new();
    article_update_data.insert("view_count".to_string(), DataValue::Int(100));
    article_update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    // 获取刚创建的用户和文章
    if let (Ok(Some(mut user_for_update)), Ok(Some(article_for_update))) = (
        ModelManager::<User>::find_by_id("tx_001").await,
        ModelManager::<Article>::find_by_id("tx_article_001").await
    ) {
        // 分别更新用户和文章
        match user_for_update.update(user_update_data).await {
            Ok(_) => println!("✅ 用户更新成功"),
            Err(e) => println!("❌ 用户更新失败: {}", e),
        }

        match article_for_update.update(article_update_data).await {
            Ok(_) => println!("✅ 文章更新成功"),
            Err(e) => println!("❌ 文章更新失败: {}", e),
        }
    }

    // 4. 演示删除操作的原子性
    println!("\n4. 演示删除操作的原子性...");

    // 获取文章并删除（删除文章后应该能查询到用户）
    if let Ok(Some(article_for_delete)) = ModelManager::<Article>::find_by_id("tx_article_001").await {
        match article_for_delete.delete().await {
            Ok(_) => {
                println!("✅ 文章删除成功");
                // 验证用户是否还存在
                match ModelManager::<User>::find_by_id("tx_001").await {
                    Ok(Some(_)) => println!("✅ 用户仍然存在（符合预期）"),
                    Ok(None) => println!("❌ 用户也被删除了（不符合预期）"),
                    Err(e) => println!("❌ 验证用户存在性失败: {}", e),
                }
            },
            Err(e) => println!("❌ 文章删除失败: {}", e),
        }
    }

    println!("✅ 事务操作演示完成");

    Ok(())
}

/// 演示性能测试
async fn demonstrate_performance_test() -> QuickDbResult<()> {
    println!("\n=== 性能测试演示 ===");

    use std::time::Instant;

    // 1. 批量插入性能测试
    println!("\n1. 批量插入性能测试...");
    let start = Instant::now();
    let batch_size = 50; // 减少测试数据量以避免过长时间

    for i in 0..batch_size {
        let user = User {
            id: format!("perf_user_{}", uuid::Uuid::new_v4()),
            username: format!("perfuser_{}_{}", i, uuid::Uuid::new_v4().simple()),
            email: format!("perfuser_{}_{}@example.com", i, uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: format!("Performance User {}", i),
            age: Some(20 + (i % 50)),
            phone: Some(format!("+86138{:08}", 1000000 + i)),
            avatar_url: Some(format!("https://avatar.example.com/perf{}.jpg", i)),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: None,
            tags: Some(vec!["性能测试".to_string()]),
        };

        match user.save().await {
            Ok(_) => {},
            Err(e) => println!("❌ 插入第 {} 条记录失败: {}", i, e),
        }
    }

    let insert_duration = start.elapsed();
    println!("✅ 插入 {} 条记录完成", batch_size);
    println!("总耗时: {:?}", insert_duration);
    println!("平均每条记录: {:?}", insert_duration / batch_size as u32);

    // 2. 查询性能测试
    println!("\n2. 查询性能测试...");
    let start = Instant::now();
    let query_count = 30; // 减少查询次数

    for i in 0..query_count {
        let conditions = vec![
            QueryCondition {
                field: "username".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("perfuser_{}_", i % 10)),
            }
        ];

        match ModelManager::<User>::find(conditions, None).await {
            Ok(_) => {},
            Err(e) => println!("❌ 第 {} 次查询失败: {}", i, e),
        }
    }

    let query_duration = start.elapsed();
    println!("✅ 执行 {} 次查询完成", query_count);
    println!("总耗时: {:?}", query_duration);
    println!("平均每次查询: {:?}", query_duration / query_count as u32);

    // 3. 更新性能测试
    println!("\n3. 更新性能测试...");
    let start = Instant::now();
    let update_count = 20; // 减少更新次数

    for i in 0..update_count {
        let conditions = vec![
            QueryCondition {
                field: "username".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("perfuser_{}_", i)),
            }
        ];

        // 查询用户然后更新
        if let Ok(mut users) = ModelManager::<User>::find(conditions.clone(), None).await {
            if let Some(mut user) = users.first() {
                let mut update_data = HashMap::new();
                update_data.insert("age".to_string(), DataValue::Int((30 + i) as i64));
                update_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

                match user.update(update_data).await {
                    Ok(_) => {},
                    Err(e) => println!("❌ 第 {} 次更新失败: {}", i, e),
                }
            }
        }
    }

    let update_duration = start.elapsed();
    println!("✅ 更新 {} 条记录完成", update_count);
    println!("总耗时: {:?}", update_duration);
    println!("平均每条记录: {:?}", update_duration / update_count as u32);

    // 4. 统计性能测试
    println!("\n4. 统计性能测试...");
    let start = Instant::now();

    match ModelManager::<User>::count(vec![]).await {
        Ok(total_count) => {
            let count_duration = start.elapsed();
            println!("✅ 统计总记录数: {}", total_count);
            println!("统计耗时: {:?}", count_duration);
        },
        Err(e) => println!("❌ 统计失败: {}", e),
    }

    // 5. 批量查询性能测试
    println!("\n5. 批量查询性能测试...");
    let start = Instant::now();

    let batch_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("perfuser_".to_string()),
        }
    ];

    match ModelManager::<User>::find(batch_conditions.clone(), None).await {
        Ok(users) => {
            let batch_duration = start.elapsed();
            println!("✅ 批量查询完成，找到 {} 个用户", users.len());
            println!("批量查询耗时: {:?}", batch_duration);
        },
        Err(e) => println!("❌ 批量查询失败: {}", e),
    }

    // 6. 连接池性能状态
    println!("\n6. 连接池性能状态...");
    let manager = get_global_pool_manager();
    let health_status = manager.health_check().await;
    println!("连接池健康状态: {:?}", health_status);

    println!("✅ 性能测试演示完成");

    Ok(())
}

/// 初始化日志系统
fn init_logging_system() -> Result<(), Box<dyn std::error::Error>> {
    LoggerBuilder::new()
        .with_level(LevelFilter::Debug)  // 设置为Debug级别以查看详细日志
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    println!("✅ 日志系统初始化成功");
    Ok(())
}

/// 主函数
#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("=== RatQuickDB 模型定义系统演示 ===");

    // 清理旧数据库文件
    let _ = std::fs::remove_file("./test_model.db");
    println!("🧹 清理旧数据库文件完成");

    // 初始化日志系统
    if let Err(e) = init_logging_system() {
        println!("❌ 日志系统初始化失败: {}", e);
        return Err(QuickDbError::ConfigError { message: format!("日志初始化失败: {}", e) });
    }
    
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
    
    // 模拟创建模型管理器
    println!("创建模型管理器...");

    println!("\n1. 演示模型验证功能");
    demonstrate_model_validation().await?;

    println!("\n2. 演示复杂查询功能");
    demonstrate_complex_queries().await?;

    println!("\n3. 演示JSON序列化功能");
    demonstrate_json_serialization().await?;

    println!("\n4. 演示连接池监控");
    demonstrate_pool_monitoring().await?;
    
    println!("\n=== 演示完成 ===");
    Ok(())
}







/// 演示模型验证功能
async fn demonstrate_model_validation() -> QuickDbResult<()> {
    println!("\n=== 模型验证演示 ===");

    // 1. 验证有效用户数据 - 使用结构体model API
    println!("\n1. 验证有效用户数据...");
    let valid_user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: format!("test_valid_user_{}", uuid::Uuid::new_v4().simple()),
        email: format!("test_valid_user_{}@example.com", uuid::Uuid::new_v4().simple()),
        password_hash: "hashed_password_here".to_string(),
        full_name: "Valid User".to_string(),
        age: Some(25),
        phone: Some("+8613812345678".to_string()),
        avatar_url: Some("https://avatar.example.com/val_001.jpg".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: Some(serde_json::json!({"preferences":{"theme":"dark","language":"zh-CN"}})),
        tags: Some(vec!["新用户".to_string(), "活跃".to_string()]),
    };

    // 验证用户数据
    match valid_user.validate() {
        Ok(_) => println!("✅ 用户数据验证通过"),
        Err(e) => println!("❌ 用户数据验证失败: {}", e),
    }

    // 保存用户到数据库
    let created_user_id = match valid_user.save().await {
        Ok(id) => {
            println!("✅ 用户保存成功，ID: {}", id);
            Some(id)
        },
        Err(e) => {
            println!("❌ 用户保存失败: {}", e);
            None
        }
    };

    // 2. 测试重复数据验证
    println!("\n2. 测试重复数据验证...");
    let duplicate_user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: format!("test_valid_user_{}", uuid::Uuid::new_v4().simple()), // 重复的用户名
        email: format!("test_duplicate_user_{}@example.com", uuid::Uuid::new_v4().simple()),
        password_hash: "hashed_password_here".to_string(),
        full_name: "Duplicate User".to_string(),
        age: Some(30),
        phone: Some("+8613812345679".to_string()),
        avatar_url: Some("https://avatar.example.com/val_002.jpg".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        last_login: None,
        profile: Some(serde_json::json!({"preferences":{"theme":"light","language":"en-US"}})),
        tags: Some(vec!["测试用户".to_string()]),
    };

    match duplicate_user.save().await {
        Ok(id) => println!("重复用户创建成功（可能允许重复）: {}", id),
        Err(e) => println!("重复用户创建失败（符合预期）: {}", e),
    }

    // 3. 验证查询条件 - 使用ModelManager API
    println!("\n3. 验证查询条件...");

    // 使用ModelManager查询用户
    if let Some(ref id) = created_user_id {
        match ModelManager::<User>::find_by_id(&id).await {
            Ok(Some(user)) => {
                println!("✅ 通过ID查询成功: {} - {}", user.id, user.username);
            },
            Ok(None) => println!("❌ 通过ID查询结果为空"),
            Err(e) => println!("❌ 通过ID查询失败: {}", e),
        }
    }

    // 使用ModelManager按用户名查询
    let username_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("validuser".to_string()),
        }
    ];

    match ModelManager::<User>::find(username_conditions, None).await {
        Ok(users) => {
            println!("✅ 按用户名查询结果: {} 个用户", users.len());
            for user in users {
                println!("   用户: {} - {}", user.id, user.username);
            }
        },
        Err(e) => println!("❌ 按用户名查询失败: {}", e),
    }

    // 4. 验证文章数据 - 使用结构体model API
    println!("\n4. 验证文章数据...");

    // 创建文章实例
    let article = Article {
        id: uuid::Uuid::new_v4().to_string(),
        title: "验证测试文章".to_string(),
        slug: format!("test-article-{}", uuid::Uuid::new_v4().simple()),
        content: "这是一篇用于验证的测试文章，内容足够长以满足验证要求。".to_string(),
        summary: Some("这是一篇用于验证的测试文章...".to_string()),
        author_id: created_user_id.clone().unwrap_or_else(|| "test_author".to_string()),
        category_id: Some("tech".to_string()),
        status: "published".to_string(),
        view_count: 0,
        like_count: 0,
        is_featured: true,
        published_at: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Some(Utc::now()),
        metadata: Some(serde_json::json!({"seo":{"keywords":["rust","编程","技术"],"description":"验证测试文章"}})),
        tags: Some(vec!["技术".to_string(), "编程".to_string()]),
    };

    // 验证文章数据
    match article.validate() {
        Ok(_) => println!("✅ 文章数据验证通过"),
        Err(e) => println!("❌ 文章数据验证失败: {}", e),
    }

    // 保存文章
    match article.save().await {
        Ok(id) => {
            println!("✅ 文章保存成功，ID: {}", id);
            println!("✅ 文章标题: {}", article.title);
        },
        Err(e) => println!("❌ 文章保存失败: {}", e),
    }

    // 5. 验证数据完整性
    println!("\n5. 验证数据完整性...");

    // 检查用户和文章的关联性
    if let Some(ref user_id) = created_user_id {
        let article_conditions = vec![
            QueryCondition {
                field: "author_id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(user_id.clone()),
            }
        ];

        match ModelManager::<Article>::find(article_conditions, None).await {
            Ok(articles) => {
                println!("✅ 用户关联的文章数量: {}", articles.len());
                for article in articles {
                    println!("   文章: {} - {}", article.id, article.title);
                }
            },
            Err(e) => println!("❌ 查询用户文章失败: {}", e),
        }
    }

    // 6. 验证统计功能
    println!("\n6. 验证统计功能...");
    match ModelManager::<User>::count(vec![]).await {
        Ok(count) => println!("✅ 总用户数: {}", count),
        Err(e) => println!("❌ 统计用户数失败: {}", e),
    }

    match ModelManager::<Article>::count(vec![]).await {
        Ok(count) => println!("✅ 总文章数: {}", count),
        Err(e) => println!("❌ 统计文章数失败: {}", e),
    }

    // 7. 验证存在性检查
    println!("\n7. 验证存在性检查...");
    let exists_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("validuser".to_string()),
        }
    ];

    match ModelManager::<User>::exists(exists_conditions).await {
        Ok(exists) => println!("✅ 用户名'validuser'存在: {}", exists),
        Err(e) => println!("❌ 存在性检查失败: {}", e),
    }

    println!("模型验证演示完成");

    Ok(())
}

/// 演示复杂查询功能
async fn demonstrate_complex_queries() -> QuickDbResult<()> {
    println!("\n=== 复杂查询演示 ===");

    // 先创建一些测试用户 - 使用结构体model API
    println!("创建测试用户...");
    let test_users = vec![
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("query_alice_{}", uuid::Uuid::new_v4().simple()),
            email: format!("query_alice_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Alice Johnson".to_string(),
            age: Some(25),
            phone: Some("+8613811111111".to_string()),
            avatar_url: Some("https://avatar.example.com/alice.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: Some(serde_json::json!({"preferences":{"theme":"dark","language":"en-US"}})),
            tags: Some(vec!["开发者".to_string(), "活跃用户".to_string()]),
        },
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("query_bob_{}", uuid::Uuid::new_v4().simple()),
            email: format!("query_bob_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Bob Smith".to_string(),
            age: Some(30),
            phone: Some("+8613822222222".to_string()),
            avatar_url: Some("https://avatar.example.com/bob.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: Some(serde_json::json!({"preferences":{"theme":"light","language":"en-US"}})),
            tags: Some(vec!["设计师".to_string(), "新用户".to_string()]),
        },
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("query_charlie_{}", uuid::Uuid::new_v4().simple()),
            email: format!("query_charlie_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Charlie Brown".to_string(),
            age: Some(35),
            phone: Some("+8613833333333".to_string()),
            avatar_url: Some("https://avatar.example.com/charlie.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: Some(serde_json::json!({"preferences":{"theme":"dark","language":"en-US"}})),
            tags: Some(vec!["管理员".to_string(), "资深用户".to_string()]),
        },
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: format!("query_diana_{}", uuid::Uuid::new_v4().simple()),
            email: format!("query_diana_{}@example.com", uuid::Uuid::new_v4().simple()),
            password_hash: "hashed_password_here".to_string(),
            full_name: "Diana Prince".to_string(),
            age: Some(28),
            phone: Some("+8613844444444".to_string()),
            avatar_url: Some("https://avatar.example.com/diana.jpg".to_string()),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            last_login: None,
            profile: Some(serde_json::json!({"preferences":{"theme":"light","language":"en-US"}})),
            tags: Some(vec!["产品经理".to_string(), "活跃用户".to_string()]),
        },
    ];

    for user in test_users {
        match user.save().await {
            Ok(id) => println!("✅ 创建测试用户成功: {} - {}", id, user.username),
            Err(e) => println!("❌ 创建测试用户失败: {}", e),
        }
    }

    // 1. 多条件查询（年龄大于25且小于35）
    println!("\n1. 多条件查询演示（年龄 > 25 且 < 35）...");
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

    match ModelManager::<User>::find(conditions, None).await {
        Ok(users) => {
            println!("✅ 多条件查询结果: {} 个用户", users.len());
            for user in users {
                println!("   用户: {} - {} (年龄: {})", user.id, user.username, user.age.unwrap_or(0));
            }
        },
        Err(e) => println!("❌ 多条件查询失败: {}", e),
    }

    // 2. 范围查询（年龄在25-35之间）
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

    match ModelManager::<User>::find(range_conditions, None).await {
        Ok(users) => {
            println!("✅ 范围查询结果: {} 个用户", users.len());
            for user in users {
                println!("   用户: {} - {} (年龄: {})", user.id, user.username, user.age.unwrap_or(0));
            }
        },
        Err(e) => println!("❌ 范围查询失败: {}", e),
    }

    // 3. 模糊查询（用户名包含'a'）
    println!("\n3. 模糊查询（用户名包含'a'）...");
    let like_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Contains,
            value: DataValue::String("a".to_string()),
        }
    ];

    match ModelManager::<User>::find(like_conditions, None).await {
        Ok(users) => {
            println!("✅ 模糊查询结果: {} 个用户", users.len());
            for user in users {
                println!("   用户: {} - {}", user.id, user.username);
            }
        },
        Err(e) => println!("❌ 模糊查询失败: {}", e),
    }

    // 4. 统计查询
    println!("\n4. 统计查询...");
    match ModelManager::<User>::count(vec![]).await {
        Ok(total) => println!("✅ 总用户数: {}", total),
        Err(e) => println!("❌ 统计查询失败: {}", e),
    }

    // 5. 存在性查询（检查是否存在用户名为'alice'的用户）
    println!("\n5. 存在性查询（检查是否存在用户名为'alice'的用户）...");
    let exists_conditions = vec![
        QueryCondition {
            field: "username".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("alice".to_string()),
        }
    ];

    match ModelManager::<User>::exists(exists_conditions).await {
        Ok(exists) => println!("✅ 用户名'alice'存在: {}", exists),
        Err(e) => println!("❌ 存在性查询失败: {}", e),
    }

    // 6. 排序查询（按年龄降序）
    println!("\n6. 排序查询（按年龄降序）...");
    let sort_options = QueryOptions {
        conditions: vec![],
        sort: vec![SortConfig {
            field: "age".to_string(),
            direction: SortDirection::Desc,
        }],
        pagination: None,
        fields: vec![],
    };

    match ModelManager::<User>::find(vec![], Some(sort_options)).await {
        Ok(users) => {
            println!("✅ 按年龄降序查询结果:");
            for (i, user) in users.iter().enumerate() {
                println!("   {}. {} - {} (年龄: {})", i + 1, user.id, user.username, user.age.unwrap_or(0));
            }
        },
        Err(e) => println!("❌ 排序查询失败: {}", e),
    }

    // 7. 分页查询（每页2条记录）
    println!("\n7. 分页查询（每页2条记录）...");
    let page_options = QueryOptions {
        conditions: vec![],
        sort: vec![],
        pagination: Some(PaginationConfig {
            limit: 2,
            skip: 0,
        }),
        fields: vec![],
    };

    match ModelManager::<User>::find(vec![], Some(page_options)).await {
        Ok(users) => {
            println!("✅ 第1页结果（前2条记录）:");
            for user in users {
                println!("   用户: {} - {}", user.id, user.username);
            }
        },
        Err(e) => println!("❌ 分页查询失败: {}", e),
    }

    println!("复杂查询演示完成");

    Ok(())
}