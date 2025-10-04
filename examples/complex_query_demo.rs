//! 复杂查询示例
//!
//! 展示如何使用AND/OR逻辑组合进行复杂查询

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryConditionGroup, LogicalOperator, QueryOperator, DataValue, QueryOptions, SortConfig, SortDirection, PaginationConfig};
use rat_quickdb::odm::find_with_groups;
use rat_quickdb::manager::{get_global_pool_manager, shutdown};
use std::collections::HashMap;
use chrono::Utc;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    rat_quickdb::init();
    println!("=== 复杂查询示例 ===");

    // 清理旧的数据库文件
    let db_files = ["/tmp/complex_query_example.db"];
    for db_path in &db_files {
        if std::path::Path::new(db_path).exists() {
            std::fs::remove_file(db_path).unwrap_or_else(|e| {
                eprintln!("警告：删除数据库文件失败 {}: {}", db_path, e);
            });
            println!("✅ 已清理旧的数据库文件: {}", db_path);
        }
    }

    // 创建数据库配置
    let config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "/tmp/complex_query_example.db".to_string(),
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
        .id_strategy(IdStrategy::AutoIncrement)
        .build()?;

    // 初始化数据库
    add_database(config).await?;

    // 创建测试表
    create_test_table().await?;

    // 插入测试数据
    insert_test_data().await?;

    println!("\n=== 开始复杂查询测试 ===\n");

    // 示例1: 简单的OR查询 - 查找年龄为25岁或者姓名为"张三"的用户
    println!("1. OR查询示例: (age = 25 OR name = '张三')");

    let or_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::Or,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int(25),
            }),
            QueryConditionGroup::Single(QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("张三".to_string()),
            }),
        ],
    };

    let or_result = find_with_groups(
        "users",
        vec![or_condition],
        None,
        None,
    ).await;

    match or_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                if let DataValue::Object(map) = user {
                    println!("   - {:?}", map);
                }
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例2: 复杂的AND和OR组合 - 查找年龄在18-30岁之间，并且(居住在北京或者职业为"开发者")的用户
    println!("2. 复杂组合查询: (age >= 18 AND age <= 30) AND (city = '北京' OR job = '开发者')");

    let complex_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 年龄条件组
            QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "age".to_string(),
                        operator: QueryOperator::Gte,
                        value: DataValue::Int(18),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "age".to_string(),
                        operator: QueryOperator::Lte,
                        value: DataValue::Int(30),
                    }),
                ],
            },
            // 地点和职业条件组
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("北京".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "job".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("开发者".to_string()),
                    }),
                ],
            },
        ],
    };

    let complex_result = find_with_groups(
        "users",
        vec![complex_condition],
        None,
        None,
    ).await;

    match complex_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                if let DataValue::Object(map) = user {
                    println!("   - {:?}", map);
                }
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例3: 嵌套的复杂条件 - 三层嵌套: AND(OR(AND), AND)
    println!("3. 嵌套查询示例: (name = '张三' OR name = '李四') AND (age > 25) AND (city = '上海' OR city = '深圳')");

    let nested_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            // 姓名条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "name".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("张三".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "name".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("李四".to_string()),
                    }),
                ],
            },
            // 年龄条件 (Single)
            QueryConditionGroup::Single(QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(25),
            }),
            // 城市条件组 (OR)
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("上海".to_string()),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "city".to_string(),
                        operator: QueryOperator::Eq,
                        value: DataValue::String("深圳".to_string()),
                    }),
                ],
            },
        ],
    };

    let nested_result = find_with_groups(
        "users",
        vec![nested_condition],
        None,
        None,
    ).await;

    match nested_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                if let DataValue::Object(map) = user {
                    println!("   - {:?}", map);
                }
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!();

    // 示例4: 带排序和分页的复杂查询
    println!("4. 带排序和分页的复杂查询: (status = 'active' AND (score > 80 OR level > 5)) 按年龄降序，限制2条");

    let sorted_condition = QueryConditionGroup::Group {
        operator: LogicalOperator::And,
        conditions: vec![
            QueryConditionGroup::Single(QueryCondition {
                field: "status".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("active".to_string()),
            }),
            QueryConditionGroup::Group {
                operator: LogicalOperator::Or,
                conditions: vec![
                    QueryConditionGroup::Single(QueryCondition {
                        field: "score".to_string(),
                        operator: QueryOperator::Gt,
                        value: DataValue::Float(80.0),
                    }),
                    QueryConditionGroup::Single(QueryCondition {
                        field: "level".to_string(),
                        operator: QueryOperator::Gt,
                        value: DataValue::Int(5),
                    }),
                ],
            },
        ],
    };

    let options = QueryOptions {
        conditions: vec![],  // 条件在condition_groups中处理
        sort: vec![
            SortConfig {
                field: "age".to_string(),
                direction: SortDirection::Desc,
            },
        ],
        pagination: Some(PaginationConfig {
            limit: 2,
            skip: 0,
        }),
        fields: vec![],
    };

    let sorted_result = find_with_groups(
        "users",
        vec![sorted_condition],
        Some(options),
        None,
    ).await;

    match sorted_result {
        Ok(users) => {
            println!("   找到 {} 个用户", users.len());
            for user in &users {
                if let DataValue::Object(map) = user {
                    println!("   - {:?}", map);
                }
            }
        },
        Err(e) => println!("   查询失败: {}", e),
    }

    println!("\n=== 复杂查询示例完成 ===");

    // 关闭连接池
    shutdown().await?;

    Ok(())
}

/// 创建测试表
async fn create_test_table() -> QuickDbResult<()> {
    use rat_quickdb::model::FieldType;

    // 定义users表的字段结构
    let mut fields = HashMap::new();

    fields.insert("name".to_string(), FieldType::String {
        max_length: Some(100),
        min_length: Some(1),
        regex: None,
    });

    fields.insert("age".to_string(), FieldType::Integer {
        min_value: Some(0),
        max_value: Some(150),
    });

    fields.insert("city".to_string(), FieldType::String {
        max_length: Some(50),
        min_length: Some(1),
        regex: None,
    });

    fields.insert("job".to_string(), FieldType::String {
        max_length: Some(50),
        min_length: Some(1),
        regex: None,
    });

    fields.insert("status".to_string(), FieldType::String {
        max_length: Some(20),
        min_length: Some(1),
        regex: None,
    });

    fields.insert("score".to_string(), FieldType::Float {
        min_value: Some(0.0),
        max_value: Some(100.0),
    });

    fields.insert("level".to_string(), FieldType::Integer {
        min_value: Some(1),
        max_value: Some(10),
    });

    fields.insert("created_at".to_string(), FieldType::DateTime);

    // 通过连接池创建表
    let manager = get_global_pool_manager();
    let pools = manager.get_connection_pools();
    if let Some(pool) = pools.get("default") {
        pool.create_table("users", &fields).await?;
    } else {
        return Err(QuickDbError::ConfigError {
            message: "无法获取默认连接池".to_string(),
        });
    }

    println!("✅ 测试表创建成功");
    Ok(())
}

/// 插入测试数据
async fn insert_test_data() -> QuickDbResult<()> {
    println!("插入测试数据...");

    let test_data = vec![
        create_user("张三", 25, "北京", "开发者", "active", 85.5, 6),
        create_user("李四", 30, "上海", "设计师", "active", 92.0, 8),
        create_user("王五", 28, "深圳", "开发者", "active", 78.0, 5),
        create_user("赵六", 32, "北京", "产品经理", "inactive", 88.0, 7),
        create_user("张三", 25, "广州", "测试", "active", 76.5, 4),
        create_user("李四", 35, "上海", "开发者", "active", 95.0, 9),
        create_user("钱七", 22, "深圳", "设计师", "active", 82.0, 6),
        create_user("孙八", 29, "北京", "开发者", "inactive", 91.5, 8),
    ];

    for (i, user_data) in test_data.iter().enumerate() {
        let result = rat_quickdb::create("users", user_data.clone(), None).await?;
        println!("   创建用户 {}: {}", i + 1, result);
    }

    println!("✅ 测试数据插入完成");
    Ok(())
}

/// 创建用户数据的辅助函数
fn create_user(name: &str, age: i32, city: &str, job: &str, status: &str, score: f64, level: i32) -> HashMap<String, DataValue> {
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String(name.to_string()));
    user_data.insert("age".to_string(), DataValue::Int(age as i64));
    user_data.insert("city".to_string(), DataValue::String(city.to_string()));
    user_data.insert("job".to_string(), DataValue::String(job.to_string()));
    user_data.insert("status".to_string(), DataValue::String(status.to_string()));
    user_data.insert("score".to_string(), DataValue::Float(score));
    user_data.insert("level".to_string(), DataValue::Int(level as i64));
    user_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));
    user_data
}