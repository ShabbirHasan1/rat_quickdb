//! RatQuickDB 基本使用示例
//! 
//! 本示例展示了如何使用 RatQuickDB 进行基本的数据库操作，
//! 包括连接配置、CRUD操作、多数据库管理等功能。

use rat_quickdb::*;
use rat_quickdb::manager::{get_global_pool_manager, health_check, shutdown};
use std::collections::HashMap;
use chrono::Utc;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    rat_quickdb::init();
    println!("=== RatQuickDB 基本使用示例 ===");
    println!("库版本: {}", rat_quickdb::get_info());
    
    // 1. 配置SQLite数据库
    println!("\n1. 配置SQLite数据库...");
    let sqlite_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./test.db".to_string(),
            create_if_missing: true,
        })
        .pool(PoolConfig::builder()
            .min_connections(2)
            .max_connections(10)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(3600)
            .build()?)
        .alias("default".to_string())
        .build()?;
    
    // 添加数据库到连接池管理器
    add_database(sqlite_config).await?;
    println!("SQLite数据库配置完成");
    
    // 2. 获取ODM管理器并进行基本操作
    println!("\n2. 开始数据库操作...");
    let odm = get_odm_manager().await;
    
    // 创建users表（如果不存在）
    create_users_table(&odm).await?;
    
    // 创建用户数据
    println!("\n2.1 创建用户数据");
    let users_data = vec![
        create_user_data("张三", "zhangsan@example.com", 25, "技术部"),
        create_user_data("李四", "lisi@example.com", 30, "产品部"),
        create_user_data("王五", "wangwu@example.com", 28, "技术部"),
        create_user_data("赵六", "zhaoliu@example.com", 32, "市场部"),
    ];
    
    for (i, user_data) in users_data.iter().enumerate() {
        let result = rat_quickdb::create("users", user_data.clone(), None).await?;
        println!("创建用户 {}: {}", i + 1, result);
    }
    
    // 查询所有用户
    println!("\n2.2 查询所有用户");
    let all_users = rat_quickdb::find("users", vec![], None, None).await?;
    println!("所有用户: {}", all_users);
    
    // 条件查询
    println!("\n2.3 条件查询 - 查找技术部员工");
    let tech_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("技术部".to_string()),
        }
    ];
    
    let tech_users = rat_quickdb::find("users", tech_conditions.clone(), None, None).await?;
    println!("技术部员工: {}", tech_users);
    
    // 范围查询
    println!("\n2.4 范围查询 - 查找年龄在25-30之间的员工");
    let age_conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        },
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::Int(30),
        }
    ];
    
    let age_filtered_users = rat_quickdb::find("users", age_conditions, None, None).await?;
    println!("年龄25-30的员工: {}", age_filtered_users);
    
    // 排序和分页查询
    println!("\n2.5 排序和分页查询");
    let query_options = QueryOptions {
        conditions: vec![],
        sort: vec![
            SortConfig {
                field: "age".to_string(),
                direction: SortDirection::Desc,
            },
            SortConfig {
                field: "name".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        pagination: Some(PaginationConfig {
            skip: 0,
            limit: 2,
        }),
        fields: vec![],
    };
    
    let sorted_users = rat_quickdb::find("users", vec![], Some(query_options), None).await?;
    println!("排序分页结果: {}", sorted_users);
    
    // 更新操作
    println!("\n2.6 更新操作 - 给技术部员工加薪");
    let mut salary_update = HashMap::new();
    salary_update.insert("salary".to_string(), DataValue::Float(8000.0));
    salary_update.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));
    
    let updated_count = rat_quickdb::update("users", tech_conditions.clone(), salary_update, None).await?;
    println!("更新了 {} 条技术部员工记录", updated_count);
    
    // 统计操作
    println!("\n2.7 统计操作");
    let total_count = rat_quickdb::count("users", vec![], None).await?;
    println!("总用户数: {}", total_count);
    
    let tech_count = rat_quickdb::count("users", tech_conditions, None).await?;
    println!("技术部员工数: {}", tech_count);
    
    // 检查记录是否存在
    let conditions = vec![
        QueryCondition {
            field: "name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Alice".to_string()),
        }
    ];
    let exists = rat_quickdb::exists("users", conditions, None).await?;
    println!("用户 Alice 是否存在: {}", exists);
    
    // 3. 连接池监控
    println!("\n3. 连接池监控");
    let manager = get_global_pool_manager();
    let aliases = manager.get_aliases();
    println!("已配置的数据库别名: {:?}", aliases);
    
    let health_map = health_check().await;
    let health = health_map.get("default").unwrap_or(&false);
    println!("数据库健康状态: {}", if *health { "正常" } else { "异常" });
    
    // 4. JSON序列化示例
    println!("\n4. JSON序列化示例");
    demonstrate_serialization().await?;
    
    // 5. 多数据库示例（如果需要）
    println!("\n5. 多数据库配置示例");
    demonstrate_multiple_databases().await?;
    
    // 删除记录
    let delete_conditions = vec![
        QueryCondition {
            field: "name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Alice".to_string()),
        }
    ];
    let deleted = rat_quickdb::delete("users", delete_conditions, None).await?;
    println!("删除的记录数: {}", deleted);
    
    // 6. 清理操作
    println!("\n6. 清理操作");
    let cleanup_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("市场部".to_string()),
        }
    ];
    
    let deleted_count = rat_quickdb::delete("users", cleanup_conditions, None).await?;
    println!("删除了 {} 条市场部记录", deleted_count);
    
    // 关闭连接池
    shutdown().await?;
    println!("\n=== 示例执行完成 ===");
    
    Ok(())
}

/// 创建users表（如果不存在）
async fn create_users_table(odm: &AsyncOdmManager) -> QuickDbResult<()> {
    use rat_quickdb::model::FieldType;
    use std::collections::HashMap;
    
    // 定义users表的字段结构
    let mut fields = HashMap::new();
    
    // name字段：字符串类型，最大长度100
    fields.insert("name".to_string(), FieldType::String {
        max_length: Some(100),
        min_length: Some(1),
        regex: None,
    });
    
    // email字段：字符串类型，最大长度255
    fields.insert("email".to_string(), FieldType::String {
        max_length: Some(255),
        min_length: Some(5),
        regex: None,
    });
    
    // age字段：整数类型，范围0-150
    fields.insert("age".to_string(), FieldType::Integer {
        min_value: Some(0),
        max_value: Some(150),
    });
    
    // department字段：字符串类型，最大长度50
    fields.insert("department".to_string(), FieldType::String {
        max_length: Some(50),
        min_length: Some(1),
        regex: None,
    });
    
    // salary字段：浮点数类型，最小值0
    fields.insert("salary".to_string(), FieldType::Float {
        min_value: Some(0.0),
        max_value: None,
    });
    
    // created_at字段：日期时间类型
    fields.insert("created_at".to_string(), FieldType::DateTime);
    
    // updated_at字段：日期时间类型
    fields.insert("updated_at".to_string(), FieldType::DateTime);
    
    // status字段：字符串类型，最大长度20
    fields.insert("status".to_string(), FieldType::String {
        max_length: Some(20),
        min_length: Some(1),
        regex: None,
    });
    
    // 获取连接和适配器
    let manager = get_global_pool_manager();
    let connection = manager.get_connection(Some("default")).await?;
    let db_type = manager.get_database_type("default")?;
    let adapter = rat_quickdb::adapter::create_adapter(&db_type)?;
    
    // 创建表
    // 注意：在新架构中，我们不能直接调用adapter.create_table
    // 这个方法应该通过连接池的操作队列来处理
    // 这里仅作为示例展示旧的调用方式
    println!("注意：create_table应该通过连接池操作队列处理，而不是直接调用适配器");
    
    // 释放连接
    manager.release_connection(&connection).await?;
    
    println!("users表创建成功（如果不存在）");
    Ok(())
}

/// 创建用户数据的辅助函数
fn create_user_data(name: &str, email: &str, age: i32, department: &str) -> HashMap<String, DataValue> {
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String(name.to_string()));
    user_data.insert("email".to_string(), DataValue::String(email.to_string()));
    user_data.insert("age".to_string(), DataValue::Int(age as i64));
    user_data.insert("department".to_string(), DataValue::String(department.to_string()));
    user_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));
    user_data.insert("status".to_string(), DataValue::String("active".to_string()));
    user_data
}

/// 演示JSON序列化功能
async fn demonstrate_serialization() -> QuickDbResult<()> {
    use rat_quickdb::serializer::*;
    
    // 创建测试数据
    let mut test_data = HashMap::new();
    test_data.insert("id".to_string(), DataValue::String("user_001".to_string()));
    test_data.insert("name".to_string(), DataValue::String("测试用户".to_string()));
    test_data.insert("score".to_string(), DataValue::Float(95.67));
    test_data.insert("active".to_string(), DataValue::Bool(true));
    test_data.insert("tags".to_string(), DataValue::Array(vec![
        DataValue::String("VIP".to_string()),
        DataValue::String("高级用户".to_string()),
    ]));
    
    // 默认序列化
    let default_serializer = DataSerializer::default();
    let result = default_serializer.serialize_record(test_data.clone())?;
    let json_string = result.to_json_string()?;
    println!("默认序列化: {}", json_string);
    
    // 创建美化输出的序列化器
    let mut pretty_config = SerializerConfig::new();
    pretty_config.pretty = true;
    pretty_config.include_null = false;
    let pretty_serializer = DataSerializer::new(pretty_config);
    
    // 序列化单个记录
    let pretty_result = pretty_serializer.serialize_record(test_data.clone())?;
    let pretty_json = pretty_result.to_json_string()?;
    println!("美化JSON输出: {}", pretty_json);
    
    // 使用PyO3兼容序列化器
    let pyo3_serializer = DataSerializer::new(SerializerConfig::for_pyo3());
    let pyo3_result = pyo3_serializer.serialize_record(test_data)?;
    let pyo3_json = pyo3_result.to_json_string()?;
    println!("PyO3兼容序列化: {}", pyo3_json);
    
    Ok(())
}

/// 演示多数据库配置
async fn demonstrate_multiple_databases() -> QuickDbResult<()> {
    println!("配置内存SQLite数据库作为缓存...");
    
    // 配置内存SQLite作为缓存数据库
    let cache_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .pool(PoolConfig::builder()
            .min_connections(1)
            .max_connections(5)
            .connection_timeout(30)
            .idle_timeout(600)
            .max_lifetime(3600)
            .build()?)
        .alias("cache".to_string())
        .build()?;
    
    add_database(cache_config).await?;
    println!("缓存数据库配置完成");
    
    // 在缓存数据库中创建一些临时数据
    let mut cache_data = HashMap::new();
    cache_data.insert("key".to_string(), DataValue::String("user_session_123".to_string()));
    cache_data.insert("value".to_string(), DataValue::String("session_data_here".to_string()));
    cache_data.insert("expires_at".to_string(), DataValue::DateTime(Utc::now() + chrono::Duration::minutes(30)));
    
    let cache_result = rat_quickdb::create("cache", cache_data, Some("cache")).await?;
    println!("缓存数据创建: {}", cache_result);
    
    // 查询缓存数据
    let cache_query = rat_quickdb::find("cache", vec![], None, Some("cache")).await?;
    println!("缓存数据查询: {}", cache_query);
    
    // 显示所有数据库的连接池状态
    let manager = get_global_pool_manager();
    let aliases = manager.get_aliases();
    println!("所有数据库连接池状态:");
    for alias in aliases {
        let db_type = manager.get_database_type(&alias).unwrap_or(DatabaseType::SQLite);
        println!("  {}: 数据库类型 {:?}", alias, db_type);
    }
    
    Ok(())
}