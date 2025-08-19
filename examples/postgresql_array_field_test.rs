use rat_quickdb::{
    array_field, string_field, integer_field,
    Model, ModelManager, ModelOperations, FieldType, DatabaseConfig, ConnectionConfig, PoolConfig, IdStrategy,
    init, add_database, DataValue, DatabaseType, QueryCondition, TlsConfig
};
use zerg_creep::{self, info, error, warn};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio;

rat_quickdb::define_model! {
    struct Student {
        id: Option<i64>,
        name: String,
        age: i32,
        scores: Vec<i32>,
        tags: Vec<String>,
        hobbies: Vec<String>,
    }
    
    collection = "students",
    database = "postgresql_test",
    fields = {
        id: integer_field(None, None),
        name: string_field(Some(100), Some(1), None).required(),
        age: integer_field(Some(0), Some(150)).required(),
        scores: array_field(
            FieldType::Integer {
                min_value: Some(0),
                max_value: Some(100),
            },
            Some(10), // 最多10个分数
            None,     // 可以没有分数
        ),
        tags: array_field(
            FieldType::String {
                max_length: Some(50),
                min_length: Some(1),
                regex: None,
            },
            Some(10), // 最多10个标签
            Some(1),  // 至少1个标签
        ),
        hobbies: array_field(
            FieldType::String {
                max_length: Some(100),
                min_length: Some(1),
                regex: None,
            },
            Some(5), // 最多5个爱好
            None,    // 可以没有爱好
        ),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    zerg_creep::init_logger();
    
    info!("开始PostgreSQL数组字段测试");
    
    // 初始化数据库
    init();
    
    // 添加PostgreSQL数据库连接
    let postgresql_config = DatabaseConfig {
        db_type: DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: Some(TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
        },
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: "postgresql_test".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    add_database(postgresql_config).await?;
    info!("PostgreSQL数据库连接添加成功");
    
    // 设置PostgreSQL为默认数据库
    use rat_quickdb::manager::get_global_pool_manager;
    get_global_pool_manager().set_default_alias("postgresql_test").await?;
    
    info!("🛡️ [MAMMOTH-READY] 开始测试PostgreSQL数组字段");
    
    // 创建测试学生数据
    let student = Student {
        id: None,
        name: "张三".to_string(),
        age: 20,
        scores: vec![95, 87, 92],
        tags: vec!["优秀".to_string(), "积极".to_string()],
        hobbies: vec!["编程".to_string(), "阅读".to_string(), "运动".to_string()],
    };
    
    info!("原始学生数据: {:?}", student);
    
    // 保存学生数据
    let student_id = student.save().await?;
    info!("学生保存成功，ID: {}", student_id);
    
    // 查询学生数据
    let found_students = ModelManager::<Student>::find(vec![], None).await?;
    info!("查询到 {} 个学生", found_students.len());
    
    for student in found_students {
        info!("学生数据: {:?}", student);
        info!("  scores: {:?}", student.scores);
        info!("  tags: {:?}", student.tags);
        info!("  hobbies: {:?}", student.hobbies);
        
        // 检查数组内容是否正确
        assert_eq!(student.scores, vec![95, 87, 92]);
        assert_eq!(student.tags, vec!["优秀".to_string(), "积极".to_string()]);
        assert_eq!(student.hobbies, vec!["编程".to_string(), "阅读".to_string(), "运动".to_string()]);
        
        info!("✅ PostgreSQL数组字段测试通过！");
    }
    
    info!("PostgreSQL数组字段测试完成");
    Ok(())
}