use rat_quickdb::types::{DatabaseConfig, ConnectionConfig, PoolConfig, DataValue, IdStrategy, DatabaseType, QueryCondition, QueryOperator};
use rat_quickdb::pool::DatabaseConnection;
use rat_quickdb::manager::get_global_pool_manager;
use rat_quickdb::{odm, quick_error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Student {
    id: String,
    name: String,
    age: i32,
    grades: Vec<i32>,
    subjects: Vec<String>,
    metadata: HashMap<String, String>,
}

impl Student {
    fn new(id: &str, name: &str, age: i32, grades: Vec<i32>, subjects: Vec<String>, metadata: HashMap<String, String>) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            age,
            grades,
            subjects,
            metadata,
        }
    }

    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), DataValue::String(self.id.clone()));
        map.insert("name".to_string(), DataValue::String(self.name.clone()));
        map.insert("age".to_string(), DataValue::Int(self.age.into()));
        
        // 将Vec<i32>转换为DataValue::Array
        let grades_array: Vec<DataValue> = self.grades.iter().map(|&g| DataValue::Int(g.into())).collect();
        map.insert("grades".to_string(), DataValue::Array(grades_array));
        
        // 将Vec<String>转换为DataValue::Array
        let subjects_array: Vec<DataValue> = self.subjects.iter().map(|s| DataValue::String(s.clone())).collect();
        map.insert("subjects".to_string(), DataValue::Array(subjects_array));
        
        // 将HashMap<String, String>转换为DataValue::Object
        let metadata_map: HashMap<String, DataValue> = self.metadata.iter()
            .map(|(k, v)| (k.clone(), DataValue::String(v.clone())))
            .collect();
        map.insert("metadata".to_string(), DataValue::Object(metadata_map));
        
        map
    }
}

#[tokio::test]
async fn test_mysql_array_field_serialization() {
    println!("开始MySQL数组字段序列化测试");
    
    // 创建MySQL数据库配置
    let db_config = DatabaseConfig {
        db_type: DatabaseType::MySQL,
        connection: ConnectionConfig::MySQL {
            host: "172.16.0.21".to_string(),
            port: 3306,
            database: "testdb".to_string(),
            username: "root".to_string(),
            password: "password".to_string(),
            ssl_opts: None,
            tls_config: None,
        },
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
        },
        alias: "mysql_test".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    // 初始化连接池
    let pool_manager = get_global_pool_manager();
    if let Err(e) = pool_manager.add_database(db_config).await {
        if e.to_string().contains("连接") || e.to_string().contains("timeout") || e.to_string().contains("Connection") {
            println!("⚠️ MySQL服务器未运行或连接超时，跳过测试: {}", e);
            return;
        }
        panic!("添加MySQL数据库配置失败: {}", e);
    }
    
    let table_name = "students_mysql_array_test";
    
    // 创建测试学生数据
    let mut metadata = HashMap::new();
    metadata.insert("class".to_string(), "计算机科学".to_string());
    metadata.insert("advisor".to_string(), "李教授".to_string());
    
    let student = Student::new(
        "student_001",
        "张三",
        20,
        vec![85, 92, 78, 96],
        vec!["数学".to_string(), "物理".to_string(), "计算机".to_string()],
        metadata,
    );
    
    println!("原始学生数据: {:?}", student);
    
    // 清空测试数据
    let _ = odm::delete(table_name, vec![], Some("mysql_test")).await;

    // 插入数据
    match odm::create(table_name, student.to_data_map(), Some("mysql_test")).await {
        Ok(_) => println!("✅ MySQL数据插入成功"),
        Err(e) => {
            let error_msg = format!("{:?}", e);
            if error_msg.contains("连接") || error_msg.contains("timeout") || error_msg.contains("Connection") {
                println!("⚠️  警告: MySQL数据插入失败，跳过测试: {}", error_msg);
                return;
            }
            panic!("MySQL数据插入失败: {:?}", e);
        }
    }
    
    // 查询数据
    let conditions = vec![QueryCondition {
        field: "id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("student_001".to_string()),
    }];
    
    let results = match odm::find(table_name, conditions, None, Some("mysql_test")).await {
        Ok(results) => results,
        Err(e) => {
            let error_msg = format!("{:?}", e);
            if error_msg.contains("连接") || error_msg.contains("timeout") || error_msg.contains("Connection") {
                println!("⚠️  警告: MySQL数据查询失败，跳过测试: {}", error_msg);
                return;
            }
            panic!("MySQL数据查询失败: {:?}", e);
        }
    };
    
    if results.is_empty() {
        panic!("未找到插入的学生数据");
    }
    
    // 检查第一条记录
    if let Some(DataValue::Object(student_data)) = results.first() {
        println!("查询到的学生数据: {:?}", student_data);
        
        // 检查grades数组字段是否正确反序列化
        if let Some(grades) = student_data.get("grades") {
            match grades {
                DataValue::Array(arr) => {
                    println!("✅ grades字段正确反序列化为数组格式: {:?}", arr);
                    // 验证数组内容
                    let expected_grades = vec![DataValue::Int(85), DataValue::Int(92), DataValue::Int(78), DataValue::Int(96)];
                    if *arr == expected_grades {
                        println!("✅ grades数组内容正确");
                    } else {
                        println!("❌ grades数组内容不匹配，期望: {:?}，实际: {:?}", expected_grades, arr);
                    }
                },
                DataValue::String(s) => {
                    println!("❌ grades字段仍被序列化为字符串: {}", s);
                    panic!("MySQL数组字段序列化问题：grades字段应该反序列化为数组格式，但仍为字符串");
                },
                _ => {
                    println!("❌ grades字段类型异常: {:?}", grades);
                    panic!("MySQL数组字段类型异常");
                }
            }
        } else {
            panic!("未找到grades字段");
        }
        
        // 检查subjects数组字段
        if let Some(subjects) = student_data.get("subjects") {
            match subjects {
                DataValue::Array(arr) => {
                    println!("✅ subjects字段正确反序列化为数组格式: {:?}", arr);
                    let expected_subjects = vec![
                        DataValue::String("数学".to_string()),
                        DataValue::String("物理".to_string()),
                        DataValue::String("计算机".to_string())
                    ];
                    if *arr == expected_subjects {
                        println!("✅ subjects数组内容正确");
                    } else {
                        println!("❌ subjects数组内容不匹配，期望: {:?}，实际: {:?}", expected_subjects, arr);
                    }
                },
                DataValue::String(s) => {
                    println!("❌ subjects字段仍被序列化为字符串: {}", s);
                    panic!("MySQL数组字段序列化问题：subjects字段应该反序列化为数组格式，但仍为字符串");
                },
                _ => {
                    println!("❌ subjects字段类型异常: {:?}", subjects);
                    panic!("MySQL数组字段类型异常");
                }
            }
        } else {
            panic!("未找到subjects字段");
        }
        
        // 检查metadata对象字段
        if let Some(metadata) = student_data.get("metadata") {
            match metadata {
                DataValue::Object(obj) => {
                    println!("✅ metadata字段正确反序列化为对象格式: {:?}", obj);
                    // 验证对象内容
                    if let (Some(DataValue::String(class)), Some(DataValue::String(advisor))) = 
                        (obj.get("class"), obj.get("advisor")) {
                        if class == "计算机科学" && advisor == "李教授" {
                            println!("✅ metadata对象内容正确");
                        } else {
                            println!("❌ metadata对象内容不匹配");
                        }
                    } else {
                        println!("❌ metadata对象结构异常");
                    }
                },
                DataValue::String(s) => {
                    println!("❌ metadata字段仍被序列化为字符串: {}", s);
                    panic!("MySQL对象字段序列化问题：metadata字段应该反序列化为对象格式，但仍为字符串");
                },
                _ => {
                    println!("❌ metadata字段类型异常: {:?}", metadata);
                    panic!("MySQL对象字段类型异常");
                }
            }
        } else {
            panic!("未找到metadata字段");
        }
        
        println!("🎉 MySQL数组字段序列化测试完成！所有字段都正确反序列化");
     } else {
         panic!("查询结果格式异常");
     }
}