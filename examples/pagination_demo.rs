//! 分页查询演示示例
//!
//! 本示例演示如何使用 QueryOptions 进行分页查询，包括：
//! - 基础分页查询
//! - 排序 + 分页组合
//! - 条件过滤 + 分页
//! - 分页导航信息计算

use rat_quickdb::{
    types::*,
    manager::{PoolManager, get_global_pool_manager},
    error::QuickDbResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Utc;

/// 用户数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
    age: i32,
    department: String,
    salary: f64,
    created_at: String,
}

impl User {
    /// 转换为数据映射
    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut data = HashMap::new();
        data.insert("name".to_string(), DataValue::String(self.name.clone()));
        data.insert("email".to_string(), DataValue::String(self.email.clone()));
        data.insert("age".to_string(), DataValue::Int(self.age as i64));
        data.insert("department".to_string(), DataValue::String(self.department.clone()));
        data.insert("salary".to_string(), DataValue::Float(self.salary));
        data.insert("created_at".to_string(), DataValue::String(self.created_at.clone()));
        data
    }

    /// 创建测试用户
    fn create_test_user(index: usize) -> Self {
        let departments = vec!["技术部", "产品部", "市场部", "销售部", "人事部"];
        let names = vec![
            "张三", "李四", "王五", "赵六", "钱七", "孙八", "周九", "吴十",
            "郑一", "王二", "冯三", "陈四", "褚五", "卫六", "蒋七", "沈八",
            "韩九", "杨十", "朱一", "秦二", "尤三", "许四", "何五", "吕六",
            "施七", "张八", "孔九", "曹十"
        ];

        Self {
            id: 0, // 数据库自动生成
            name: format!("{}{}", names[index % names.len()], index + 1),
            email: format!("user{}@company.com", index + 1),
            age: (index % 35) + 22, // 22-56岁
            department: departments[index % departments.len()].to_string(),
            salary: 5000.0 + (index % 20) as f64 * 1000.0, // 5000-24000
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// 分页信息
#[derive(Debug)]
struct PageInfo {
    current_page: u64,
    page_size: u64,
    total_items: u64,
    total_pages: u64,
    has_next: bool,
    has_prev: bool,
}

impl PageInfo {
    fn new(current_page: u64, page_size: u64, total_items: u64) -> Self {
        let total_pages = (total_items + page_size - 1) / page_size;
        let has_next = current_page < total_pages;
        let has_prev = current_page > 1;

        Self {
            current_page,
            page_size,
            total_items,
            total_pages,
            has_next,
            has_prev,
        }
    }

    fn display(&self) {
        println!("📄 分页信息:");
        println!("   当前页: {}/{}", self.current_page, self.total_pages);
        println!("   页面大小: {}", self.page_size);
        println!("   总记录数: {}", self.total_items);
        println!("   总页数: {}", self.total_pages);
        println!("   上一页: {}", if self.has_prev { "✓" } else { "✗" });
        println!("   下一页: {}", if self.has_next { "✓" } else { "✗" });
    }
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🚀 RatQuickDB 分页查询演示");
    println!("=============================\n");

    // 1. 配置数据库
    println!("1. 配置SQLite数据库...");
    let db_config = DatabaseConfig {
        alias: "main".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            database: "./pagination_demo.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::AutoIncrement,
        cache: None,
    };

    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;
    println!("✅ 数据库配置完成\n");

    // 2. 创建表
    println!("2. 创建用户表...");
    let create_table_result = rat_quickdb::create_table(
        "users",
        &[
            ("name", FieldType::String),
            ("email", FieldType::String),
            ("age", FieldType::Integer),
            ("department", FieldType::String),
            ("salary", FieldType::Float),
            ("created_at", FieldType::String),
        ],
        Some("main"),
    ).await;

    match create_table_result {
        Ok(_) => println!("✅ 用户表创建成功"),
        Err(_) => println!("ℹ️  用户表可能已存在"),
    }
    println!();

    // 3. 插入测试数据
    println!("3. 插入测试数据...");

    // 先清空现有数据
    let _ = rat_quickdb::delete("users", vec![], Some("main")).await;

    let users: Vec<User> = (0..50).map(|i| User::create_test_user(i)).collect();
    let mut created_count = 0;

    for user in &users {
        let data = user.to_data_map();
        match rat_quickdb::create("users", data, Some("main")).await {
            Ok(_) => created_count += 1,
            Err(e) => println!("❌ 创建用户失败: {}", e),
        }
    }

    println!("✅ 成功创建 {} 个用户", created_count);
    println!();

    // 4. 演示基础分页查询
    println!("4. 🔍 基础分页查询");
    println!("==================");

    let page_size = 5;
    let total_count = rat_quickdb::count("users", vec![], Some("main")).await?;
    let total_pages = (total_count + page_size - 1) / page_size;

    println!("总共 {} 条记录，每页 {} 条，共 {} 页\n", total_count, page_size, total_pages);

    for page in 1..=std::cmp::min(3, total_pages) {
        let skip = (page - 1) * page_size;

        let query_options = QueryOptions {
            conditions: vec![],
            sort: vec![],
            pagination: Some(PaginationConfig {
                skip,
                limit: page_size,
            }),
            fields: vec![],
        };

        let page_users = rat_quickdb::find("users", vec![], Some(query_options), Some("main")).await?;

        let page_info = PageInfo::new(page, page_size, total_count);

        println!("--- 第 {} 页 ---", page);
        page_info.display();
        println!("📋 用户列表:");

        for (index, user) in page_users.iter().enumerate() {
            if let DataValue::Object(user_map) = user {
                let name = user_map.get("name").and_then(|v| {
                    if let DataValue::String(s) = v { Some(s) } else { None }
                }).unwrap_or(&"未知".to_string());

                let age = user_map.get("age").and_then(|v| {
                    if let DataValue::Int(i) = v { Some(*i) } else { None }
                }).unwrap_or(0);

                let department = user_map.get("department").and_then(|v| {
                    if let DataValue::String(s) = v { Some(s) } else { None }
                }).unwrap_or(&"未知".to_string());

                println!("   {}. {} ({}岁, {})", index + 1, name, age, department);
            }
        }
        println!();
    }

    if total_pages > 3 {
        println!("... 还有 {} 页数据 ...\n", total_pages - 3);
    }

    // 5. 演示排序 + 分页
    println!("5. 🔄 排序 + 分页查询");
    println!("===================");

    let sort_query_options = QueryOptions {
        conditions: vec![],
        sort: vec![
            SortConfig {
                field: "salary".to_string(),
                direction: SortDirection::Desc,
            },
            SortConfig {
                field: "name".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        pagination: Some(PaginationConfig {
            skip: 0,
            limit: 8,
        }),
        fields: vec![],
    };

    let high_salary_users = rat_quickdb::find("users", vec![], Some(sort_query_options), Some("main")).await?;

    println!("📊 按薪资降序、姓名升序排列的前8名用户:");
    for (index, user) in high_salary_users.iter().enumerate() {
        if let DataValue::Object(user_map) = user {
            let name = user_map.get("name").and_then(|v| {
                if let DataValue::String(s) = v { Some(s) } else { None }
            }).unwrap_or(&"未知".to_string());

            let salary = user_map.get("salary").and_then(|v| {
                if let DataValue::Float(f) = v { Some(*f) } else { None }
            }).unwrap_or(0.0);

            let department = user_map.get("department").and_then(|v| {
                if let DataValue::String(s) = v { Some(s) } else { None }
            }).unwrap_or(&"未知".to_string());

            println!("   {}. {} - 薪资: {:.2} - {}", index + 1, name, salary, department);
        }
    }
    println!();

    // 6. 演示条件过滤 + 分页
    println!("6. 🔍 条件过滤 + 分页查询");
    println!("=======================");

    // 查询技术部年龄大于30岁的用户
    let filter_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("技术部".to_string()),
        },
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(30),
        },
    ];

    let filter_count = rat_quickdb::count("users", filter_conditions.clone(), Some("main")).await?;

    println!("📋 查询条件: 技术部且年龄大于30岁");
    println!("📊 符合条件的用户数: {}\n", filter_count);

    if filter_count > 0 {
        let filter_query_options = QueryOptions {
            conditions: filter_conditions.clone(),
            sort: vec![
                SortConfig {
                    field: "age".to_string(),
                    direction: SortDirection::Desc,
                }
            ],
            pagination: Some(PaginationConfig {
                skip: 0,
                limit: 10,
            }),
            fields: vec![],
        };

        let filtered_users = rat_quickdb::find("users", filter_conditions, Some(filter_query_options), Some("main")).await?;

        println!("👥 技术部年龄大于30岁的用户 (按年龄降序):");
        for (index, user) in filtered_users.iter().enumerate() {
            if let DataValue::Object(user_map) = user {
                let name = user_map.get("name").and_then(|v| {
                    if let DataValue::String(s) = v { Some(s) } else { None }
                }).unwrap_or(&"未知".to_string());

                let age = user_map.get("age").and_then(|v| {
                    if let DataValue::Int(i) = v { Some(*i) } else { None }
                }).unwrap_or(0);

                let salary = user_map.get("salary").and_then(|v| {
                    if let DataValue::Float(f) = v { Some(*f) } else { None }
                }).unwrap_or(0.0);

                println!("   {}. {} - {}岁 - 薪资: {:.2}", index + 1, name, age, salary);
            }
        }
    }
    println!();

    // 7. 演示字段选择 + 分页
    println!("7. 📝 字段选择 + 分页查询");
    println!("=======================");

    let fields_query_options = QueryOptions {
        conditions: vec![],
        sort: vec![],
        pagination: Some(PaginationConfig {
            skip: 10,
            limit: 5,
        }),
        fields: vec!["name".to_string(), "department".to_string(), "salary".to_string()],
    };

    let selected_fields_users = rat_quickdb::find("users", vec![], Some(fields_query_options), Some("main")).await?;

    println!("📋 跳过前10条，只显示姓名、部门、薪资字段:");
    for (index, user) in selected_fields_users.iter().enumerate() {
        if let DataValue::Object(user_map) = user {
            let name = user_map.get("name").and_then(|v| {
                if let DataValue::String(s) = v { Some(s) } else { None }
            }).unwrap_or(&"未知".to_string());

            let department = user_map.get("department").and_then(|v| {
                if let DataValue::String(s) = v { Some(s) } else { None }
            }).unwrap_or(&"未知".to_string());

            let salary = user_map.get("salary").and_then(|v| {
                if let DataValue::Float(f) = v { Some(*f) } else { None }
            }).unwrap_or(0.0);

            println!("   {}. {} - {} - 薪资: {:.2}", index + 11, name, department, salary);
        }
    }
    println!();

    // 8. 清理
    println!("8. 🧹 清理演示数据");
    println!("===================");

    // 删除测试数据
    let deleted_count = rat_quickdb::delete("users", vec![], Some("main")).await?;
    println!("✅ 删除了 {} 条测试记录", deleted_count);

    // 关闭连接池
    rat_quickdb::shutdown().await?;

    println!("\n🎉 分页查询演示完成！");
    Ok(())
}