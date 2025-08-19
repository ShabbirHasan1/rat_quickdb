use rat_quickdb::{
    array_field, list_field, string_field, integer_field, float_field, boolean_field, datetime_field, dict_field,
    Model, ModelManager, ModelOperations, FieldType, DatabaseConfig, ConnectionConfig, PoolConfig, IdStrategy,
    init, add_database, DataValue, DatabaseType, QueryCondition
};
use zerg_creep::{self, info, error};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio;

rat_quickdb::define_model! {
    struct Student {
        id: Option<i64>,
        name: String,
        age: i32,
        scores: Vec<i32>,
        grades: Vec<f64>,
        is_active: Vec<bool>,
        tags: Vec<String>,
        hobbies: Vec<String>,
        #[serde(with = "rat_quickdb::types::serde_helpers::hashmap_datavalue")]
        metadata: HashMap<String, DataValue>,
    }
    
    collection = "students",
    database = "mysql_test",
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
        grades: array_field(
            FieldType::Float {
                min_value: Some(0.0),
                max_value: Some(4.0),
            },
            Some(10), // 最多10个成绩
            None,     // 可以没有成绩
        ),
        is_active: array_field(
            FieldType::Boolean,
            Some(5),  // 最多5个布尔值
            None,     // 可以没有
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
        metadata: dict_field({
            let mut fields = HashMap::new();
            fields.insert("school".to_string(), string_field(Some(100), None, None));
            fields.insert("year".to_string(), integer_field(Some(2000), Some(2030)));
            fields.insert("gpa".to_string(), float_field(Some(0.0), Some(4.0)));
            fields.insert("graduated".to_string(), boolean_field());
            fields
        }),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    zerg_creep::init_logger();
    
    info!("开始MySQL数组字段测试");
    
    // 初始化数据库
    init();
    
    // 添加MySQL数据库连接
    let mysql_config = DatabaseConfig {
        db_type: DatabaseType::MySQL,
        connection: ConnectionConfig::MySQL {
            host: "172.16.0.21".to_string(),
            port: 3306,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_opts: {
                let mut opts = HashMap::new();
                opts.insert("ssl_mode".to_string(), "PREFERRED".to_string());
                Some(opts)
            },
            tls_config: None,
        },
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: "mysql_test".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    add_database(mysql_config).await?;
    info!("MySQL数据库连接添加成功");
    
    // 设置MySQL为默认数据库
    use rat_quickdb::manager::get_global_pool_manager;
    let pool_manager = get_global_pool_manager();
    pool_manager.set_default_alias("mysql_test").await?;
    
    // 通过PoolManager获取连接池来删除可能存在的残留表，确保测试环境干净
    info!("清理测试环境，删除可能存在的残留表...");
    let pools = pool_manager.get_connection_pools();
    if let Some(pool) = pools.get("mysql_test") {
        let _ = pool.drop_table("students").await; // 忽略错误，表可能不存在
    }
    info!("测试环境清理完成");
    
    // 创建测试学生数据
    let mut metadata = HashMap::new();
    metadata.insert("school".to_string(), DataValue::String("北京大学".to_string()));
    metadata.insert("year".to_string(), DataValue::Int(2023));
    metadata.insert("gpa".to_string(), DataValue::Float(3.7));
    metadata.insert("graduated".to_string(), DataValue::Bool(true));
    
    let student = Student {
        id: None,
        name: "张三".to_string(),
        age: 20,
        scores: vec![95, 87, 92],
        grades: vec![3.7, 3.4, 3.8],
        is_active: vec![true, true, false],
        tags: vec!["优秀".to_string(), "积极".to_string()],
        hobbies: vec!["编程".to_string(), "阅读".to_string(), "运动".to_string()],
        metadata,
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
        info!("  grades: {:?}", student.grades);
        info!("  is_active: {:?}", student.is_active);
        info!("  tags: {:?}", student.tags);
        info!("  hobbies: {:?}", student.hobbies);
        info!("  metadata: {:?}", student.metadata);
        
        // 检查数组内容是否正确
        assert_eq!(student.scores, vec![95, 87, 92]);
        assert_eq!(student.grades, vec![3.7, 3.4, 3.8]);
        assert_eq!(student.is_active, vec![true, true, false]);
        assert_eq!(student.tags, vec!["优秀".to_string(), "积极".to_string()]);
        assert_eq!(student.hobbies, vec!["编程".to_string(), "阅读".to_string(), "运动".to_string()]);
        
        // 检查字典内容是否正确
        assert_eq!(student.metadata.get("school"), Some(&DataValue::String("北京大学".to_string())));
        assert_eq!(student.metadata.get("year"), Some(&DataValue::Int(2023)));
        assert_eq!(student.metadata.get("gpa"), Some(&DataValue::Float(3.7)));
        assert_eq!(student.metadata.get("graduated"), Some(&DataValue::Bool(true)));
        
        info!("✅ MySQL数组字段和字典字段测试通过！");
    }
    
    info!("MySQL数组字段测试完成");
    Ok(())
}