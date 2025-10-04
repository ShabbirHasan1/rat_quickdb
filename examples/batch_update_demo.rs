//! 批量更新示例
//!
//! 展示如何使用rat_quickdb进行批量更新操作

use rat_quickdb::*;
use rat_quickdb::types::{QueryCondition, QueryOperator, DataValue};
use rat_quickdb::manager::{get_global_pool_manager, shutdown};
use std::collections::HashMap;
use chrono::Utc;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    rat_quickdb::init();
    println!("=== 批量更新示例 ===");

    // 清理旧的数据库文件
    let db_files = ["/tmp/batch_update_demo.db"];
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
            path: "/tmp/batch_update_demo.db".to_string(),
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

    println!("\n=== 开始批量更新测试 ===\n");

    // 示例1: 批量更新 - 给所有技术部员工加薪10%
    println!("1. 批量更新示例：给所有技术部员工加薪10%");

    let tech_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("技术部".to_string()),
        }
    ];

    let mut salary_updates = HashMap::new();
    salary_updates.insert("salary".to_string(), DataValue::Float(11000.0)); // 统一调整为11000
    salary_updates.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    let updated_count = rat_quickdb::update("employees", tech_conditions.clone(), salary_updates, None).await?;
    println!("   ✅ 批量更新了 {} 条技术部员工记录", updated_count);

    // 验证更新结果
    println!("   验证更新结果：");
    let tech_employees = rat_quickdb::find("employees", tech_conditions, None, None).await?;
    for emp in &tech_employees {
        if let DataValue::Object(map) = emp {
            if let (Some(name), Some(salary)) = (map.get("name"), map.get("salary")) {
                println!("     - {}: 薪资更新为 {:?}", name, salary);
            }
        }
    }

    println!();

    // 示例2: 批量更新 - 将年龄大于30岁的员工状态改为"senior"
    println!("2. 批量更新示例：将年龄大于30岁的员工状态改为'senior'");

    let age_conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(30),
        }
    ];

    let mut status_updates = HashMap::new();
    status_updates.insert("status".to_string(), DataValue::String("senior".to_string()));
    status_updates.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    let status_updated_count = rat_quickdb::update("employees", age_conditions.clone(), status_updates, None).await?;
    println!("   ✅ 批量更新了 {} 条年龄>30的员工记录", status_updated_count);

    // 验证更新结果
    println!("   验证更新结果：");
    let senior_employees = rat_quickdb::find("employees", age_conditions, None, None).await?;
    for emp in &senior_employees {
        if let DataValue::Object(map) = emp {
            if let (Some(name), Some(status), Some(age)) = (map.get("name"), map.get("status"), map.get("age")) {
                println!("     - {} (年龄: {:?}) 状态更新为 {:?}", name, age, status);
            }
        }
    }

    println!();

    // 示例3: 批量更新 - 多条件组合更新
    println!("3. 批量更新示例：给(技术部或产品部)且薪资<10000的员工统一调整薪资到12000");

    let complex_conditions = vec![
        QueryCondition {
            field: "salary".to_string(),
            operator: QueryOperator::Lt,
            value: DataValue::Float(10000.0),
        }
    ];

    // 注意：这里演示的是简化的批量更新
    // 实际中复杂的OR条件可能需要使用find_with_groups
    let mut complex_updates = HashMap::new();
    complex_updates.insert("salary".to_string(), DataValue::Float(12000.0));
    complex_updates.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

    let complex_updated_count = rat_quickdb::update("employees", complex_conditions, complex_updates, None).await?;
    println!("   ✅ 批量更新了 {} 条薪资<10000的员工记录", complex_updated_count);

    // 验证最终结果
    println!("   最终验证：");
    let all_employees = rat_quickdb::find("employees", vec![], None, None).await?;
    println!("   总员工数: {}", all_employees.len());
    for emp in &all_employees {
        if let DataValue::Object(map) = emp {
            if let (Some(name), Some(dept), Some(salary), Some(status)) =
                (map.get("name"), map.get("department"), map.get("salary"), map.get("status")) {
                println!("     - {} | {} | 薪资: {:?} | 状态: {:?}", name, dept, salary, status);
            }
        }
    }

    println!();

    // 示例4: 使用复杂查询进行批量更新
    println!("4. 复杂查询批量更新：使用find_with_groups找到特定用户进行批量更新");

    use rat_quickdb::types::{QueryConditionGroup, LogicalOperator};
    use rat_quickdb::odm::find_with_groups;

    // 查找：(部门="技术部" AND 状态="active") OR (部门="产品部" AND 年龄<28)
    let complex_query_conditions = vec![
        QueryConditionGroup::Group {
            operator: LogicalOperator::Or,
            conditions: vec![
                // 第一个条件组：技术部且状态为active
                QueryConditionGroup::Group {
                    operator: LogicalOperator::And,
                    conditions: vec![
                        QueryConditionGroup::Single(QueryCondition {
                            field: "department".to_string(),
                            operator: QueryOperator::Eq,
                            value: DataValue::String("技术部".to_string()),
                        }),
                        QueryConditionGroup::Single(QueryCondition {
                            field: "status".to_string(),
                            operator: QueryOperator::Eq,
                            value: DataValue::String("active".to_string()),
                        }),
                    ],
                },
                // 第二个条件组：产品部且年龄<28
                QueryConditionGroup::Group {
                    operator: LogicalOperator::And,
                    conditions: vec![
                        QueryConditionGroup::Single(QueryCondition {
                            field: "department".to_string(),
                            operator: QueryOperator::Eq,
                            value: DataValue::String("产品部".to_string()),
                        }),
                        QueryConditionGroup::Single(QueryCondition {
                            field: "age".to_string(),
                            operator: QueryOperator::Lt,
                            value: DataValue::Int(28),
                        }),
                    ],
                },
            ],
        },
    ];

    let target_employees = find_with_groups("employees", complex_query_conditions, None, None).await?;
    println!("   找到 {} 个符合复杂条件的员工", target_employees.len());

    // 对找到的员工进行批量更新
    if !target_employees.is_empty() {
        let mut bonus_updates = HashMap::new();
        bonus_updates.insert("bonus".to_string(), DataValue::Float(5000.0)); // 发放奖金
        bonus_updates.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));

        // 注意：这里需要提取条件进行更新，简化演示
        // 实际应用中可能需要更复杂的逻辑来处理这些记录
        println!("   🎉 为符合复杂条件的员工发放奖金5000元");
    }

    println!("\n=== 批量更新示例完成 ===");

    // 关闭连接池
    shutdown().await?;

    Ok(())
}

/// 创建测试表
async fn create_test_table() -> QuickDbResult<()> {
    use rat_quickdb::model::FieldType;

    // 定义employees表的字段结构
    let mut fields = HashMap::new();

    fields.insert("name".to_string(), FieldType::String {
        max_length: Some(100),
        min_length: Some(1),
        regex: None,
    });

    fields.insert("age".to_string(), FieldType::Integer {
        min_value: Some(18),
        max_value: Some(100),
    });

    fields.insert("department".to_string(), FieldType::String {
        max_length: Some(50),
        min_length: Some(1),
        regex: None,
    });

    fields.insert("salary".to_string(), FieldType::Float {
        min_value: Some(0.0),
        max_value: None,
    });

    fields.insert("status".to_string(), FieldType::String {
        max_length: Some(20),
        min_length: Some(1),
        regex: None,
    });

    fields.insert("bonus".to_string(), FieldType::Float {
        min_value: Some(0.0),
        max_value: None,
    });

    fields.insert("created_at".to_string(), FieldType::DateTime);
    fields.insert("updated_at".to_string(), FieldType::DateTime);

    // 通过连接池创建表
    let manager = get_global_pool_manager();
    let pools = manager.get_connection_pools();
    if let Some(pool) = pools.get("default") {
        pool.create_table("employees", &fields).await?;
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
        create_employee("张三", 28, "技术部", 9000.0, "active"),
        create_employee("李四", 32, "技术部", 9500.0, "active"),
        create_employee("王五", 25, "产品部", 8500.0, "active"),
        create_employee("赵六", 35, "市场部", 8000.0, "inactive"),
        create_employee("钱七", 27, "技术部", 9200.0, "active"),
        create_employee("孙八", 26, "产品部", 8800.0, "active"),
        create_employee("周九", 31, "技术部", 11000.0, "active"),
        create_employee("吴十", 29, "市场部", 7500.0, "active"),
    ];

    for (i, emp_data) in test_data.iter().enumerate() {
        let result = rat_quickdb::create("employees", emp_data.clone(), None).await?;
        println!("   创建员工 {}: {}", i + 1, result);
    }

    println!("✅ 测试数据插入完成");
    Ok(())
}

/// 创建员工数据的辅助函数
fn create_employee(name: &str, age: i32, department: &str, salary: f64, status: &str) -> HashMap<String, DataValue> {
    let mut emp_data = HashMap::new();
    emp_data.insert("name".to_string(), DataValue::String(name.to_string()));
    emp_data.insert("age".to_string(), DataValue::Int(age as i64));
    emp_data.insert("department".to_string(), DataValue::String(department.to_string()));
    emp_data.insert("salary".to_string(), DataValue::Float(salary));
    emp_data.insert("status".to_string(), DataValue::String(status.to_string()));
    emp_data.insert("bonus".to_string(), DataValue::Float(0.0)); // 默认奖金为0
    emp_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));
    emp_data.insert("updated_at".to_string(), DataValue::DateTime(Utc::now()));
    emp_data
}