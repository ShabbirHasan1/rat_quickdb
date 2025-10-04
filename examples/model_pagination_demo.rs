//! 模型分页查询演示示例
//!
//! 本示例演示如何使用 define_model! 宏和 ModelManager 进行分页查询，包括：
//! - 使用宏定义模型
//! - ModelManager 的各种分页查询
//! - 排序 + 分页组合
//! - 条件过滤 + 分页
//! - 字段选择 + 分页

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, DataValue, QueryCondition, QueryOperator, SortDirection, SortConfig, PaginationConfig};
use rat_quickdb::manager::{get_global_pool_manager, health_check};
use rat_quickdb::{ModelManager, ModelOperations, string_field, integer_field, float_field, boolean_field, datetime_field};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use std::collections::HashMap;
use std::time::Duration;
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};

// 定义员工模型
define_model! {
    /// 员工模型
    struct Employee {
        id: String,
        employee_id: String,
        name: String,
        email: String,
        department: String,
        position: String,
        age: i32,
        salary: f64,
        hire_date: chrono::DateTime<chrono::Utc>,
        is_active: bool,
        skills: Option<Vec<String>>,
        performance_rating: Option<f64>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }
    collection = "employees",
    fields = {
        id: string_field(None, None, None).required().unique(),
        employee_id: string_field(None, None, None).required().unique(),
        name: string_field(None, None, None).required(),
        email: string_field(None, None, None).required(),
        department: string_field(None, None, None).required(),
        position: string_field(None, None, None).required(),
        age: integer_field(None, None).required(),
        salary: float_field(None, None).required(),
        hire_date: datetime_field().required(),
        is_active: boolean_field().required(),
        skills: array_field(field_types!(string), None, None),
        performance_rating: float_field(None, None),
        created_at: datetime_field().required(),
        updated_at: datetime_field(),
    }
    indexes = [
        { fields: ["employee_id"], unique: true, name: "idx_employee_id" },
        { fields: ["department"], unique: false, name: "idx_department" },
        { fields: ["salary"], unique: false, name: "idx_salary" },
        { fields: ["is_active", "department"], unique: false, name: "idx_active_department" },
        { fields: ["hire_date"], unique: false, name: "idx_hire_date" },
    ],
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

/// 创建测试员工数据
fn create_test_employees(count: usize) -> Vec<Employee> {
    let departments = vec![
        "技术部", "产品部", "市场部", "销售部", "人事部",
        "财务部", "运营部", "客服部", "研发部", "设计部"
    ];

    let positions = vec![
        "初级工程师", "中级工程师", "高级工程师", "技术专家", "技术总监",
        "产品经理", "产品总监", "市场专员", "市场经理", "销售代表",
        "销售经理", "人事专员", "人事经理", "财务专员", "财务经理",
        "运营专员", "运营经理", "客服专员", "客服主管", "UI设计师"
    ];

    let first_names = vec![
        "张", "李", "王", "赵", "钱", "孙", "周", "吴", "郑", "王", "冯", "陈",
        "褚", "卫", "蒋", "沈", "韩", "杨", "朱", "秦", "尤", "许", "何", "吕"
    ];

    let last_names = vec![
        "伟", "芳", "娜", "秀英", "敏", "静", "丽", "强", "磊", "军", "洋", "勇",
        "艳", "杰", "涛", "明", "超", "秀兰", "霞", "平", "刚", "桂英", "小红"
    ];

    let skills_options = vec![
        vec!["Rust", "Python", "JavaScript"],
        vec!["Java", "Spring", "MySQL"],
        vec!["React", "Vue", "TypeScript"],
        vec!["Docker", "Kubernetes", "AWS"],
        vec!["项目管理", "团队协作", "沟通"],
        vec!["数据分析", "Excel", "SQL"],
        vec!["市场营销", "品牌推广", "客户关系"],
        vec!["财务分析", "成本控制", "预算管理"],
        vec!["UI设计", "UX设计", "Figma"],
        vec!["测试", "质量保证", "自动化"]
    ];

    let mut employees = Vec::new();

    for i in 0..count {
        let employee_id = format!("EMP{:04}", i + 1);
        let name = format!("{}{}",
            first_names[i % first_names.len()],
            last_names[i % last_names.len()]
        );
        let email = format!("{}.{}@company.com",
            name.chars().take(1).collect::<String>().to_lowercase(),
            employee_id.to_lowercase()
        );

        let created_at = Utc::now() - Duration::from_secs((i * 86400) as u64); // 每天一个员工
        let hire_date = created_at - Duration::from_secs((i * 3600) as u64); // 每小时相差

        // 测试ID自动生成 - 不设置id字段，让系统自动生成
        let employee = Employee {
            id: String::new(), // 设置为空，测试ODM的ID自动生成逻辑
            employee_id,
            name,
            email,
            department: departments[i % departments.len()].to_string(),
            position: positions[i % positions.len()].to_string(),
            age: ((i % 35) + 22) as i32, // 22-56岁
            salary: 5000.0 + (i % 30) as f64 * 1000.0, // 5000-34000
            hire_date,
            is_active: true,
            skills: Some(skills_options[i % skills_options.len()].iter().map(|s| s.to_string()).collect()),
            performance_rating: Some(3.0 + (i % 7) as f64 * 0.5), // 3.0-6.0
            created_at,
            updated_at: None,
        };

        employees.push(employee);
    }

    employees
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🚀 RatQuickDB 模型分页查询演示");
    println!("==============================\n");

    // 清理之前的测试文件
    cleanup_test_files().await;

    // 1. 配置数据库
    println!("1. 配置SQLite数据库...");
    let db_config = DatabaseConfig {
        alias: "main".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./model_pagination_demo.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;
    println!("✅ 数据库配置完成\n");

    // 2. 健康检查
    println!("2. 数据库健康检查...");
    let health_results = health_check().await;
    if let Some(&is_healthy) = health_results.get("main") {
        if is_healthy {
            println!("✅ 数据库连接正常");
        } else {
            println!("❌ 数据库连接异常");
            return Err(QuickDbError::ConnectionError {
                message: "数据库连接异常".to_string(),
            });
        }
    } else {
        println!("❌ 未找到main数据库配置");
        return Err(QuickDbError::ConnectionError {
            message: "未找到main数据库配置".to_string(),
        });
    }
    println!();

    // 3. 创建测试数据
    println!("3. 创建测试员工数据...");
    let test_employees = create_test_employees(50);

    // 先清空现有数据
    let _ = rat_quickdb::delete("employees", vec![], Some("main")).await;

    let mut created_count = 0;
    for employee in &test_employees {
        match employee.save().await {
            Ok(_) => created_count += 1,
            Err(e) => println!("❌ 创建员工失败: {}", e),
        }
    }

    println!("✅ 成功创建 {} 个员工", created_count);
    println!();

    // 4. 基础分页查询演示
    println!("4. 🔍 基础分页查询演示");
    println!("======================");

    let page_size = 8;
    let total_count = ModelManager::<Employee>::count(vec![]).await?;
    let total_pages = (total_count + page_size - 1) / page_size;

    println!("📊 总共 {} 条记录，每页 {} 条，共 {} 页\n", total_count, page_size, total_pages);

    // 显示前3页
    for page in 1..=std::cmp::min(3, total_pages) {
        let skip = (page - 1) * page_size;

        let page_options = QueryOptions {
            conditions: vec![],
            sort: vec![],
            pagination: Some(PaginationConfig {
                limit: page_size,
                skip,
            }),
            fields: vec![],
        };

        match ModelManager::<Employee>::find(vec![], Some(page_options)).await {
            Ok(employees) => {
                let page_info = PageInfo::new(page, page_size, total_count);

                println!("--- 第 {} 页 ---", page);
                page_info.display();
                println!("👥 员工列表:");

                for (index, employee) in employees.iter().enumerate() {
                    println!("   {}. {} - {} ({}) - {} - 薪资: {:.2}",
                        index + 1,
                        employee.employee_id,
                        employee.name,
                        employee.age,
                        employee.department,
                        employee.salary
                    );
                }
                println!();
            },
            Err(e) => println!("❌ 第{}页查询失败: {}", page, e),
        }
    }

    if total_pages > 3 {
        println!("... 还有 {} 页数据 ...\n", total_pages - 3);
    }

    // 5. 排序 + 分页查询演示
    println!("5. 🔄 排序 + 分页查询演示");
    println!("========================");

    // 按薪资降序、年龄升序排序
    let sort_options = QueryOptions {
        conditions: vec![],
        sort: vec![
            SortConfig {
                field: "salary".to_string(),
                direction: SortDirection::Desc,
            },
            SortConfig {
                field: "age".to_string(),
                direction: SortDirection::Asc,
            },
        ],
        pagination: Some(PaginationConfig {
            limit: 10,
            skip: 0,
        }),
        fields: vec![],
    };

    match ModelManager::<Employee>::find(vec![], Some(sort_options)).await {
        Ok(employees) => {
            println!("📊 按薪资降序、年龄升序的前10名员工:");
            for (index, employee) in employees.iter().enumerate() {
                let rating = employee.performance_rating.unwrap_or(0.0);
                println!("   {}. {} - {} - {}岁 - 薪资: {:.2} - 绩效: {:.1}",
                    index + 1,
                    employee.name,
                    employee.position,
                    employee.age,
                    employee.salary,
                    rating
                );
            }
        },
        Err(e) => println!("❌ 排序查询失败: {}", e),
    }
    println!();

    // 6. 条件过滤 + 分页查询演示
    println!("6. 🔍 条件过滤 + 分页查询演示");
    println!("===========================");

    // 查询技术部薪资大于15000的员工
    let filter_conditions = vec![
        QueryCondition {
            field: "department".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("技术部".to_string()),
        },
        QueryCondition {
            field: "salary".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Float(15000.0),
        },
    ];

    let filter_count = ModelManager::<Employee>::count(filter_conditions.clone()).await?;
    println!("📋 查询条件: 技术部且薪资 > 15000");
    println!("📊 符合条件的员工数: {}\n", filter_count);

    if filter_count > 0 {
        let filter_options = QueryOptions {
            conditions: filter_conditions.clone(),
            sort: vec![
                SortConfig {
                    field: "salary".to_string(),
                    direction: SortDirection::Desc,
                }
            ],
            pagination: Some(PaginationConfig {
                limit: 15,
                skip: 0,
            }),
            fields: vec![],
        };

        match ModelManager::<Employee>::find(filter_conditions, Some(filter_options)).await {
            Ok(employees) => {
                println!("👥 技术部高薪员工 (按薪资降序):");
                for (index, employee) in employees.iter().enumerate() {
                    println!("   {}. {} - {} - 薪资: {:.2} - {}",
                        index + 1,
                        employee.name,
                        employee.position,
                        employee.salary,
                        employee.employee_id
                    );
                }
            },
            Err(e) => println!("❌ 条件查询失败: {}", e),
        }
    }
    println!();

    // 7. 字段选择 + 分页查询演示
    println!("7. 📝 字段选择 + 分页查询演示");
    println!("===========================");

    let fields_options = QueryOptions {
        conditions: vec![],
        sort: vec![
            SortConfig {
                field: "hire_date".to_string(),
                direction: SortDirection::Desc,
            }
        ],
        pagination: Some(PaginationConfig {
            limit: 8,
            skip: 5, // 跳过前5条
        }),
        fields: vec![
            "employee_id".to_string(),
            "name".to_string(),
            "department".to_string(),
            "position".to_string(),
            "hire_date".to_string()
        ],
    };

    match ModelManager::<Employee>::find(vec![], Some(fields_options)).await {
        Ok(employees) => {
            println!("📋 跳过前5条，只显示员工ID、姓名、部门、职位、入职日期:");
            for (index, employee) in employees.iter().enumerate() {
                println!("   {}. {} - {} - {} - {} - 入职: {}",
                    index + 6,
                    employee.employee_id,
                    employee.name,
                    employee.department,
                    employee.position,
                    employee.hire_date.format("%Y-%m-%d").to_string()
                );
            }
        },
        Err(e) => println!("❌ 字段选择查询失败: {}", e),
    }
    println!();

    // 8. 复杂分页查询演示
    println!("8. 🔬 复杂分页查询演示");
    println!("====================");

    // 查询活跃员工，年龄25-40岁，按部门和绩效排序
    let complex_conditions = vec![
        QueryCondition {
            field: "is_active".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Bool(true),
        },
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        },
        QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Lte,
            value: DataValue::Int(40),
        },
    ];

    let complex_count = ModelManager::<Employee>::count(complex_conditions.clone()).await?;
    println!("📊 25-40岁活跃员工数: {}", complex_count);

    if complex_count > 0 {
        let complex_options = QueryOptions {
            conditions: complex_conditions.clone(),
            sort: vec![
                SortConfig {
                    field: "department".to_string(),
                    direction: SortDirection::Asc,
                },
                SortConfig {
                    field: "performance_rating".to_string(),
                    direction: SortDirection::Desc,
                },
                SortConfig {
                    field: "salary".to_string(),
                    direction: SortDirection::Desc,
                },
            ],
            pagination: Some(PaginationConfig {
                limit: 20,
                skip: 0,
            }),
            fields: vec![],
        };

        match ModelManager::<Employee>::find(complex_conditions, Some(complex_options)).await {
            Ok(employees) => {
                println!("👥 25-40岁活跃员工 (按部门分组，绩效和薪资降序):");
                let mut current_dept = String::new();
                for (index, employee) in employees.iter().enumerate() {
                    if employee.department != current_dept {
                        current_dept = employee.department.clone();
                        println!("   📍 {}:", current_dept);
                    }

                    let rating = employee.performance_rating.unwrap_or(0.0);
                    let skills_str = employee.skills.as_ref()
                        .map(|skills| skills.join(", "))
                        .unwrap_or_else(|| "无".to_string());

                    println!("     {}. {} - {}岁 - 薪资: {:.2} - 绩效: {:.1} - 技能: {}",
                        index + 1,
                        employee.name,
                        employee.age,
                        employee.salary,
                        rating,
                        skills_str
                    );
                }
            },
            Err(e) => println!("❌ 复杂查询失败: {}", e),
        }
    }
    println!();

    // 9. 性能测试 - 不同页面大小的查询性能
    println!("9. ⚡ 分页性能测试");
    println!("==================");

    let page_sizes = vec![5, 10, 20, 50];

    for &page_size in &page_sizes {
        let start = std::time::Instant::now();

        let performance_options = QueryOptions {
            conditions: vec![],
            sort: vec![
                SortConfig {
                    field: "salary".to_string(),
                    direction: SortDirection::Desc,
                }
            ],
            pagination: Some(PaginationConfig {
                limit: page_size,
                skip: 0,
            }),
            fields: vec!["name".to_string(), "salary".to_string(), "department".to_string()],
        };

        match ModelManager::<Employee>::find(vec![], Some(performance_options)).await {
            Ok(employees) => {
                let duration = start.elapsed();
                println!("📊 页面大小 {}: {} 条记录, 耗时: {:?} (平均: {:.2}ms/条)",
                    page_size,
                    employees.len(),
                    duration,
                    duration.as_millis() as f64 / employees.len() as f64
                );
            },
            Err(e) => println!("❌ 性能测试失败 (页面大小 {}): {}", page_size, e),
        }
    }
    println!();

    // 10. 清理演示数据
    println!("10. 🧹 清理演示数据");
    println!("==================");

    match rat_quickdb::delete("employees", vec![], Some("main")).await {
        Ok(count) => println!("✅ 删除了 {} 条测试记录", count),
        Err(e) => println!("❌ 清理失败: {}", e),
    }

    // 关闭连接池
    shutdown().await?;

    // 清理测试文件
    cleanup_test_files().await;

    println!("\n🎉 模型分页查询演示完成！");
    Ok(())
}

/// 清理测试文件
async fn cleanup_test_files() {
    let test_files = vec![
        "./model_pagination_demo.db",
        "./model_pagination_demo.db-wal",
        "./model_pagination_demo.db-shm",
    ];

    for file in test_files {
        if let Err(e) = tokio::fs::remove_file(file).await {
            // 忽略文件不存在的错误
            if !e.to_string().contains("No such file or directory") {
                eprintln!("警告：无法删除测试文件 {}: {}", file, e);
            }
        }
    }
}