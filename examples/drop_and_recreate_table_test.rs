//! 删除并重建表功能测试示例
//! 
//! 演示如何使用 rat_quickdb 的删除并重建表功能
//! 支持 PostgreSQL、MySQL、SQLite 和 MongoDB

use std::collections::HashMap;
use rat_quickdb::{
    add_database, get_connection, release_connection, shutdown,
    TableManager, TableSchema, ColumnDefinition, ColumnType, TableCreateOptions,
    FieldType, DataValue, Model, ModelOperations,
    string_field, integer_field, boolean_field, array_field,
    postgres_config,
    GlobalConfigBuilder, DatabaseConfigBuilder, PoolConfigBuilder,
    QuickDbResult, QuickDbError,
};
use zerg_creep::{info, error, init_logger};
use std::sync::Arc;

/// 学生模型定义
#[derive(Debug, Clone)]
struct Student {
    pub id: Option<String>,
    pub name: String,
    pub age: i32,
    pub is_active: bool,
    pub scores: Vec<f64>,
}

impl Model for Student {
    fn get_table_name() -> &'static str {
        "students"
    }
    
    fn get_fields() -> HashMap<String, FieldType> {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), string_field(Some(100), false));
        fields.insert("age".to_string(), integer_field(false));
        fields.insert("is_active".to_string(), boolean_field(false));
        fields.insert("scores".to_string(), array_field(FieldType::Float, false));
        fields
    }
}

impl Student {
    pub fn new(name: String, age: i32, is_active: bool, scores: Vec<f64>) -> Self {
        Self {
            id: None,
            name,
            age,
            is_active,
            scores,
        }
    }
    
    pub fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut map = HashMap::new();
        if let Some(ref id) = self.id {
            map.insert("id".to_string(), DataValue::String(id.clone()));
        }
        map.insert("name".to_string(), DataValue::String(self.name.clone()));
        map.insert("age".to_string(), DataValue::Integer(self.age as i64));
        map.insert("is_active".to_string(), DataValue::Boolean(self.is_active));
        map.insert("scores".to_string(), DataValue::Array(self.scores.iter().map(|&x| DataValue::Float(x)).collect()));
        map
    }
}

/// 创建表模式
fn create_table_schema() -> TableSchema {
    let mut columns = HashMap::new();
    
    columns.insert("id".to_string(), ColumnDefinition {
        name: "id".to_string(),
        column_type: ColumnType::String,
        nullable: false,
        default_value: None,
        max_length: Some(36),
        precision: None,
        scale: None,
        auto_increment: false,
        primary_key: true,
        unique: true,
        indexed: true,
        foreign_key: None,
        check_constraint: None,
        comment: Some("主键ID".to_string()),
    });
    
    columns.insert("name".to_string(), ColumnDefinition {
        name: "name".to_string(),
        column_type: ColumnType::String,
        nullable: false,
        default_value: None,
        max_length: Some(100),
        precision: None,
        scale: None,
        auto_increment: false,
        primary_key: false,
        unique: false,
        indexed: true,
        foreign_key: None,
        check_constraint: None,
        comment: Some("学生姓名".to_string()),
    });
    
    columns.insert("age".to_string(), ColumnDefinition {
        name: "age".to_string(),
        column_type: ColumnType::Integer,
        nullable: false,
        default_value: None,
        max_length: None,
        precision: None,
        scale: None,
        auto_increment: false,
        primary_key: false,
        unique: false,
        indexed: false,
        foreign_key: None,
        check_constraint: Some("age > 0 AND age < 150".to_string()),
        comment: Some("学生年龄".to_string()),
    });
    
    columns.insert("is_active".to_string(), ColumnDefinition {
        name: "is_active".to_string(),
        column_type: ColumnType::Boolean,
        nullable: false,
        default_value: Some(DataValue::Boolean(true)),
        max_length: None,
        precision: None,
        scale: None,
        auto_increment: false,
        primary_key: false,
        unique: false,
        indexed: false,
        foreign_key: None,
        check_constraint: None,
        comment: Some("是否活跃".to_string()),
    });
    
    columns.insert("scores".to_string(), ColumnDefinition {
        name: "scores".to_string(),
        column_type: ColumnType::Json,
        nullable: true,
        default_value: None,
        max_length: None,
        precision: None,
        scale: None,
        auto_increment: false,
        primary_key: false,
        unique: false,
        indexed: false,
        foreign_key: None,
        check_constraint: None,
        comment: Some("成绩数组".to_string()),
    });
    
    TableSchema {
        name: "students".to_string(),
        columns,
        indexes: HashMap::new(),
        constraints: HashMap::new(),
        version: 1,
        description: Some("学生信息表".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// 测试删除并重建表功能
async fn test_drop_and_recreate_table() -> QuickDbResult<()> {
    info!("开始测试删除并重建表功能");
    
    // 配置数据库连接
    let global_config = GlobalConfigBuilder::new()
        .app_name("drop_recreate_test".to_string())
        .version("1.0.0".to_string())
        .environment(rat_quickdb::Environment::Development)
        .build()?;
    
    let db_config = DatabaseConfigBuilder::new()
        .host("localhost".to_string())
        .port(5432)
        .username("postgres".to_string())
        .password("password".to_string())
        .database_name("test_db".to_string())
        .build()?;
    
    let pool_config = PoolConfigBuilder::new()
        .max_connections(10)
        .min_connections(2)
        .connection_timeout_seconds(30)
        .idle_timeout_seconds(600)
        .max_lifetime_seconds(1800)
        .build()?;
    
    // 添加数据库连接
    add_database(
        "default".to_string(),
        postgres_config(db_config),
        pool_config,
        global_config,
    ).await?;
    
    // 创建表管理器
    let pool_manager = rat_quickdb::manager::get_pool_manager().await
        .ok_or_else(|| QuickDbError::ConnectionError {
            message: "无法获取连接池管理器".to_string(),
        })?;
    
    let table_manager = Arc::new(TableManager::new(
        pool_manager,
        rat_quickdb::table::TableManagerConfig::default(),
    ));
    
    // 创建表模式
    let schema = create_table_schema();
    
    info!("检查表是否存在");
    let table_exists = table_manager.check_table_exists("students").await?;
    info!("表 students 存在状态: {}", table_exists);
    
    // 如果表存在，先插入一些测试数据
    if table_exists {
        info!("表已存在，插入测试数据");
        let connection = get_connection(Some("default")).await?;
        
        let student1 = Student::new(
            "张三".to_string(),
            20,
            true,
            vec![85.5, 92.0, 78.5],
        );
        
        let student2 = Student::new(
            "李四".to_string(),
            22,
            false,
            vec![90.0, 88.5, 95.0],
        );
        
        // 插入数据
        let _id1 = student1.create(&connection).await?;
        let _id2 = student2.create(&connection).await?;
        
        info!("成功插入测试数据");
        
        release_connection(&connection).await?;
    }
    
    // 执行删除并重建表操作
    info!("执行删除并重建表操作");
    let options = TableCreateOptions {
        if_not_exists: false, // 强制重建
        create_indexes: true,
        add_constraints: true,
        table_options: None,
    };
    
    table_manager.drop_and_recreate_table(&schema, Some(options)).await?;
    info!("删除并重建表操作完成");
    
    // 验证表已重建
    info!("验证表是否重建成功");
    let table_exists_after = table_manager.check_table_exists("students").await?;
    info!("重建后表 students 存在状态: {}", table_exists_after);
    
    if !table_exists_after {
        return Err(QuickDbError::Other(anyhow::anyhow!("表重建失败")));
    }
    
    // 插入新数据验证表功能正常
    info!("插入新数据验证表功能");
    let connection = get_connection(Some("default")).await?;
    
    let new_student = Student::new(
        "王五".to_string(),
        21,
        true,
        vec![88.0, 91.5, 87.0],
    );
    
    let new_id = new_student.create(&connection).await?;
    info!("成功插入新学生数据，ID: {}", new_id);
    
    // 查询验证
    let found_student = Student::find_by_id(&connection, &new_id).await?;
    if let Some(student_data) = found_student {
        info!("查询到学生数据: {:?}", student_data);
    } else {
        return Err(QuickDbError::Other(anyhow::anyhow!("查询新插入的学生数据失败")));
    }
    
    release_connection(&connection).await?;
    
    info!("删除并重建表功能测试完成");
    Ok(())
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    init_logger();
    
    info!("启动删除并重建表测试");
    
    match test_drop_and_recreate_table().await {
        Ok(_) => {
            info!("✅ 删除并重建表测试成功完成");
        }
        Err(e) => {
            error!("❌ 删除并重建表测试失败: {}", e);
            return Err(e);
        }
    }
    
    // 清理资源
    shutdown().await?;
    info!("测试程序退出");
    
    Ok(())
}