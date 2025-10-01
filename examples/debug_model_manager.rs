//! ModelManager调试示例 - 专注于find方法问题
//!
//! 这个示例专门用于排查ModelManager::find失败的原因

use rat_quickdb::*;
use rat_quickdb::model::*;
use rat_quickdb::types::*;
use rat_quickdb::manager::get_global_pool_manager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 使用宏定义测试模型 - 包含复杂字段
define_model! {
    /// 测试用户模型
    struct TestUser {
        id: String,
        name: String,
        email: String,
        age: i32,
        salary: f64,
        active: bool,
        department: String,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
        profile: Option<serde_json::Value>,
        tags: Option<Vec<String>>,
        score: Option<i32>,
    }
    collection = "test_users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        email: string_field(None, None, None).required(),
        age: integer_field(None, None).required(),
        salary: float_field(None, None).required(),
        active: boolean_field().required(),
        department: string_field(None, None, None).required(),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
        profile: json_field(),
        tags: array_field(field_types!(string), None, None),
        score: integer_field(None, None),
    }
    indexes = [
        { fields: ["id"], unique: true, name: "idx_id" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["department"], unique: false, name: "idx_department" },
        { fields: ["active"], unique: false, name: "idx_active" },
        { fields: ["salary"], unique: false, name: "idx_salary" },
        { fields: ["created_at"], unique: false, name: "idx_created_at" },
        { fields: ["active", "department"], unique: false, name: "idx_active_dept" },
    ],
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("=== ModelManager find方法调试 ===");

    // 1. 初始化系统
    init();
    let _ = std::fs::remove_file("./debug_test.db");

    // 2. 创建数据库配置
    let db_config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./debug_test.db".to_string(),
            create_if_missing: true,
        })
        .pool(PoolConfig::default())
        .alias("default".to_string())
        .id_strategy(IdStrategy::AutoIncrement)
        .build()?;

    // 3. 添加数据库
    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // 4. 创建测试数据（完整字段）
    let test_users = vec![
        TestUser {
            id: "user_001".to_string(),
            name: "张三".to_string(),
            email: "zhangsan@company.com".to_string(),
            age: 25,
            salary: 15000.50,
            active: true,
            department: "技术部".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: Some(chrono::Utc::now()),
            profile: Some(serde_json::json!({
                "position": "高级工程师",
                "skills": ["Rust", "Go", "Python"],
                "experience_years": 3
            })),
            tags: Some(vec!["前端".to_string(), "后端".to_string(), "全栈".to_string()]),
            score: Some(85),
        },
        TestUser {
            id: "user_002".to_string(),
            name: "李四".to_string(),
            email: "lisi@company.com".to_string(),
            age: 30,
            salary: 22000.00,
            active: false,
            department: "产品部".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: None,
            profile: Some(serde_json::json!({
                "position": "产品经理",
                "skills": ["产品设计", "用户研究", "数据分析"],
                "experience_years": 5
            })),
            tags: Some(vec!["产品".to_string(), "设计".to_string()]),
            score: Some(92),
        },
        TestUser {
            id: "user_003".to_string(),
            name: "王五".to_string(),
            email: "wangwu@company.com".to_string(),
            age: 28,
            salary: 18000.75,
            active: true,
            department: "技术部".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: Some(chrono::Utc::now()),
            profile: Some(serde_json::json!({
                "position": "架构师",
                "skills": ["架构设计", "微服务", "云原生"],
                "experience_years": 6
            })),
            tags: Some(vec!["架构".to_string(), "云原生".to_string()]),
            score: Some(95),
        },
        TestUser {
            id: "user_004".to_string(),
            name: "赵六".to_string(),
            email: "zhaoliu@company.com".to_string(),
            age: 35,
            salary: 28000.00,
            active: true,
            department: "市场部".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: None,
            profile: Some(serde_json::json!({
                "position": "市场总监",
                "skills": ["市场策略", "品牌管理", "商务拓展"],
                "experience_years": 8
            })),
            tags: Some(vec!["市场".to_string(), "管理".to_string()]),
            score: Some(88),
        },
    ];

    // 5. 创建数据
    println!("创建测试数据...");
    for user in &test_users {
        match user.save().await {
            Ok(id) => println!("✅ 创建成功: {} ({})", user.name, id),
            Err(e) => {
                println!("❌ 创建失败 {}: {}", user.name, e);
                return Err(e);
            }
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // 6. 核心测试：ModelManager::find
    println!("\n=== 核心测试：ModelManager::find ===");

    // 6.1 测试空条件查询
    println!("6.1 测试空条件查询...");
    match ModelManager::<TestUser>::find(Vec::new(), None).await {
        Ok(users) => {
            println!("✅ 空条件查询成功，找到 {} 个用户", users.len());
            for user in &users {
                println!("   - {} ({}) - {}岁 - {}", user.name, user.id, user.age, if user.active { "激活" } else { "未激活" });
            }
        }
        Err(e) => {
            println!("❌ 空条件查询失败: {}", e);
            println!("错误详情: {:?}", e);

            // 尝试分析错误
            match &e {
                QuickDbError::ValidationError { field, message } => {
                    println!("   -> 字段验证错误: field={}, message={}", field, message);
                }
                QuickDbError::ConnectionError { message } => {
                    println!("   -> 连接错误: {}", message);
                }
                QuickDbError::SerializationError { message } => {
                    println!("   -> 序列化错误: {}", message);
                }
                _ => {
                    println!("   -> 其他错误: {}", e);
                }
            }
        }
    }

    // 6.2 测试带条件的查询
    println!("\n6.2 测试带条件的查询...");

    // 查找激活用户
    let active_condition = QueryCondition {
        field: "active".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::Bool(true),
    };

    match ModelManager::<TestUser>::find(vec![active_condition], None).await {
        Ok(users) => {
            println!("✅ 条件查询成功 (active=true)，找到 {} 个用户", users.len());
            for user in users {
                println!("   - {} ({})", user.name, user.id);
            }
        }
        Err(e) => {
            println!("❌ 条件查询失败: {}", e);
        }
    }

    // 6.3 测试年龄条件查询
    let age_condition = QueryCondition {
        field: "age".to_string(),
        operator: QueryOperator::Gte,
        value: DataValue::Int(25),
    };

    match ModelManager::<TestUser>::find(vec![age_condition], None).await {
        Ok(users) => {
            println!("✅ 年龄条件查询成功 (age>=25)，找到 {} 个用户", users.len());
            for user in users {
                println!("   - {} ({}) - {}岁", user.name, user.id, user.age);
            }
        }
        Err(e) => {
            println!("❌ 年龄条件查询失败: {}", e);
        }
    }

    // 6.4 测试复合条件查询
    let complex_conditions = vec![
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        },
        QueryCondition {
            field: "active".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Bool(true),
        },
    ];

    match ModelManager::<TestUser>::find(complex_conditions, None).await {
        Ok(users) => {
            println!("✅ 复合条件查询成功 (age>=25 && active=true)，找到 {} 个用户", users.len());
            for user in users {
                println!("   - {} ({}) - {}岁 - {} - 薪资: {}", user.name, user.id, user.age, user.department, user.salary);
            }
        }
        Err(e) => {
            println!("❌ 复合条件查询失败: {}", e);
        }
    }

    // 6.5 测试薪资条件查询
    println!("\n6.5 测试薪资条件查询...");
    let salary_condition = QueryCondition {
        field: "salary".to_string(),
        operator: QueryOperator::Gt,
        value: DataValue::Float(20000.0),
    };

    match ModelManager::<TestUser>::find(vec![salary_condition], None).await {
        Ok(users) => {
            println!("✅ 薪资条件查询成功 (salary>20000)，找到 {} 个用户", users.len());
            for user in users {
                println!("   - {} ({}) - {} - 薪资: {}", user.name, user.id, user.department, user.salary);
            }
        }
        Err(e) => {
            println!("❌ 薪资条件查询失败: {}", e);
        }
    }

    // 6.6 测试部门条件查询
    println!("\n6.6 测试部门条件查询...");
    let dept_condition = QueryCondition {
        field: "department".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("技术部".to_string()),
    };

    match ModelManager::<TestUser>::find(vec![dept_condition], None).await {
        Ok(users) => {
            println!("✅ 部门条件查询成功 (department=技术部)，找到 {} 个用户", users.len());
            for user in users {
                println!("   - {} ({}) - {}岁 - 薪资: {}", user.name, user.id, user.age, user.salary);
            }
        }
        Err(e) => {
            println!("❌ 部门条件查询失败: {}", e);
        }
    }

    // 7. 核心测试：更新功能验证 - 用户强调的关键功能
    println!("\n=== 核心测试：更新功能验证 ===");

    // 7.1 先查询一个用户用于更新测试
    let update_target_condition = QueryCondition {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("user_001".to_string()),
    };

    match ModelManager::<TestUser>::find(vec![update_target_condition], None).await {
        Ok(mut users) => {
            if let Some(mut target_user) = users.pop() {
                println!("✅ 找到更新目标用户: {} ({})", target_user.name, target_user.id);

                // 记录原始数据
                let original_salary = target_user.salary;
                let original_dept = target_user.department.clone();
                println!("   原始薪资: {}, 原始部门: {}", original_salary, original_dept);

                // 7.2 执行更新操作
                let mut updates = HashMap::new();
                updates.insert("salary".to_string(), DataValue::Float(18000.0));
                updates.insert("department".to_string(), DataValue::String("研发部".to_string()));
                updates.insert("updated_at".to_string(), DataValue::String(chrono::Utc::now().to_rfc3339()));

                println!("执行更新: 薪资 {} -> {}, 部门 {} -> {}",
                    original_salary, 18000.0, original_dept, "研发部");

                match target_user.update(updates).await {
                    Ok(updated) => {
                        if updated {
                            println!("✅ 更新操作成功");

                            // 7.3 验证更新结果
                            let verify_condition = QueryCondition {
                                field: "id".to_string(),
                                operator: QueryOperator::Eq,
                                value: DataValue::String("user_001".to_string()),
                            };

                            match ModelManager::<TestUser>::find(vec![verify_condition], None).await {
                                Ok(mut verify_users) => {
                                    if let Some(verified_user) = verify_users.pop() {
                                        println!("✅ 验证更新结果:");
                                        println!("   新薪资: {} (原: {})", verified_user.salary, original_salary);
                                        println!("   新部门: {} (原: {})", verified_user.department, original_dept);
                                        println!("   更新时间: {:?}", verified_user.updated_at);

                                        // 验证数据确实更新了
                                        if verified_user.salary == 18000.0 && verified_user.department == "研发部" {
                                            println!("✅ 数据更新验证成功！");
                                        } else {
                                            println!("❌ 数据更新验证失败！");
                                        }
                                    } else {
                                        println!("❌ 更新后查询不到用户");
                                    }
                                }
                                Err(e) => {
                                    println!("❌ 更新验证查询失败: {}", e);
                                }
                            }
                        } else {
                            println!("❌ 更新操作失败");
                        }
                    }
                    Err(e) => {
                        println!("❌ 更新操作异常: {}", e);
                    }
                }
            } else {
                println!("❌ 未找到更新目标用户");
            }
        }
        Err(e) => {
            println!("❌ 更新目标查询失败: {}", e);
        }
    }

    // 7.4 批量更新测试
    println!("\n7.4 批量更新测试 - 为所有技术部员工加薪...");
    let tech_dept_condition = QueryCondition {
        field: "department".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("技术部".to_string()),
    };

    match ModelManager::<TestUser>::find(vec![tech_dept_condition], None).await {
        Ok(tech_users) => {
            println!("找到 {} 个技术部员工，开始批量更新...", tech_users.len());

            let mut update_count = 0;
            for mut tech_user in tech_users {
                let mut batch_updates = HashMap::new();
                batch_updates.insert("salary".to_string(), DataValue::Float(tech_user.salary * 1.1)); // 加薪10%
                batch_updates.insert("updated_at".to_string(), DataValue::String(chrono::Utc::now().to_rfc3339()));

                match tech_user.update(batch_updates).await {
                    Ok(updated) => {
                        if updated {
                            update_count += 1;
                            println!("✅ {} 加薪成功，新薪资: {:.2}", tech_user.name, tech_user.salary * 1.1);
                        }
                    }
                    Err(e) => {
                        println!("❌ {} 加薪失败: {}", tech_user.name, e);
                    }
                }
            }

            println!("批量更新完成，成功更新 {} 个用户", update_count);
        }
        Err(e) => {
            println!("❌ 批量更新查询失败: {}", e);
        }
    }

    // 7. 手动调试：直接查询数据库
    println!("\n=== 手动调试：直接查询数据库 ===");

    let odm_manager = get_odm_manager().await;

    // 直接查询所有数据
    match odm_manager.find("test_users", Vec::new(), None, Some("default")).await {
        Ok(result) => {
            println!("✅ 直接查询成功");
            println!("返回结果类型: {}", std::any::type_name_of_val(&result));

            // 尝试序列化结果为JSON
            match serde_json::to_string(&result) {
                Ok(json_str) => {
                    println!("✅ JSON序列化成功");
                    println!("结果长度: {} 字符", json_str.len());

                    // 尝试解析回JSON
                    match serde_json::from_str::<serde_json::Value>(&json_str) {
                        Ok(json_value) => {
                            println!("✅ JSON解析成功");
                            if let Some(array) = json_value.as_array() {
                                println!("找到 {} 条记录", array.len());
                            }
                        }
                        Err(e) => {
                            println!("❌ JSON解析失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ JSON序列化失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ 直接查询失败: {}", e);
        }
    }

    // 8. 验证宏系统是否正常工作
    println!("\n=== 宏系统验证 ===");

    // 创建一个测试用户并验证to_data_map/from_data_map
    let test_user = TestUser {
        id: "macro_test".to_string(),
        name: "宏测试".to_string(),
        email: "macro_test@example.com".to_string(),
        age: 99,
        salary: 99999.99,
        active: true,
        department: "测试部".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: Some(chrono::Utc::now()),
        profile: Some(serde_json::json!({"role": "tester"})),
        tags: Some(vec!["测试".to_string()]),
        score: Some(100),
    };

    // 验证to_data_map
    match test_user.to_data_map() {
        Ok(data_map) => {
            println!("✅ to_data_map正常，生成 {} 个字段", data_map.len());

            // 验证from_data_map
            match TestUser::from_data_map(data_map) {
                Ok(reconstructed) => {
                    println!("✅ from_data_map正常");
                    if reconstructed.id == test_user.id && reconstructed.name == test_user.name {
                        println!("✅ 数据往返转换完全一致");
                    } else {
                        println!("❌ 数据往返转换不一致");
                    }
                }
                Err(e) => {
                    println!("❌ from_data_map失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ to_data_map失败: {}", e);
        }
    }

    println!("\n=== 调试完成 ===");
    Ok(())
}