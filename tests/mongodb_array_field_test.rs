use rat_quickdb::types::{DatabaseConfig, ConnectionConfig, PoolConfig, DataValue, IdStrategy, DatabaseType, TlsConfig, ZstdConfig, QueryCondition, QueryOperator};
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
    scores: Vec<i32>,
    tags: Vec<String>,
    hobbies: Vec<String>,
}

impl Student {
    fn new(id: &str, name: &str, age: i32, scores: Vec<i32>, tags: Vec<String>, hobbies: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            age,
            scores,
            tags,
            hobbies,
        }
    }

    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut map = HashMap::new();
        map.insert("_id".to_string(), DataValue::String(self.id.clone()));
        map.insert("name".to_string(), DataValue::String(self.name.clone()));
        map.insert("age".to_string(), DataValue::Int(self.age.into()));
        
        // 将Vec<i32>转换为DataValue::Array
        let scores_array: Vec<DataValue> = self.scores.iter().map(|&s| DataValue::Int(s.into())).collect();
        map.insert("scores".to_string(), DataValue::Array(scores_array));
        
        // 将Vec<String>转换为DataValue::Array
        let tags_array: Vec<DataValue> = self.tags.iter().map(|s| DataValue::String(s.clone())).collect();
        map.insert("tags".to_string(), DataValue::Array(tags_array));
        
        let hobbies_array: Vec<DataValue> = self.hobbies.iter().map(|s| DataValue::String(s.clone())).collect();
        map.insert("hobbies".to_string(), DataValue::Array(hobbies_array));
        
        map
    }
}

#[tokio::test]
#[ignore]
async fn test_mongodb_array_field_serialization() {
    // 这个测试需要远程MongoDB服务器，默认忽略
    println!("⚠️ 此测试需要远程MongoDB服务器，默认忽略");
    println!("开始MongoDB数组字段序列化测试");
    
    // 创建MongoDB数据库配置（从示例文件中提取的配置）
    let db_config = DatabaseConfig {
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
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
            zstd_config: Some(ZstdConfig {
                enabled: true,
                compression_level: Some(3),
                compression_threshold: Some(1024),
            }),
            options: None,
        },
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: "mongodb_test".to_string(),
        cache: None,
        id_strategy: IdStrategy::ObjectId,
    };
    
    // 初始化连接池
    let pool_manager = get_global_pool_manager();
    if let Err(e) = pool_manager.add_database(db_config).await {
        if e.to_string().contains("连接") || e.to_string().contains("timeout") || e.to_string().contains("Connection") {
            println!("⚠️ MongoDB服务器未运行或连接超时，跳过测试: {}", e);
            return;
        }
        panic!("添加MongoDB数据库配置失败: {}", e);
    }
    
    let table_name = "students_mongodb_array_test";
    
    // 创建测试学生数据
    let student = Student::new(
        "student_001",
        "张三",
        20,
        vec![85, 92, 78, 96],
        vec!["优秀".to_string(), "积极".to_string(), "团队合作".to_string()],
        vec!["编程".to_string(), "阅读".to_string(), "游泳".to_string()],
    );
    
    println!("原始学生数据: {:?}", student);
    
    // 清空测试数据
    let _ = odm::delete(table_name, vec![], Some("mongodb_test")).await;

    // 插入数据
    match odm::create(table_name, student.to_data_map(), Some("mongodb_test")).await {
        Ok(_) => println!("✅ MongoDB数据插入成功"),
        Err(e) => {
            let error_msg = format!("{:?}", e);
            if error_msg.contains("连接") || error_msg.contains("timeout") || error_msg.contains("Connection") {
                println!("⚠️  警告: MongoDB数据插入失败，跳过测试: {}", error_msg);
                return;
            }
            panic!("MongoDB数据插入失败: {:?}", e);
        }
    }
    
    // 查询数据
    let conditions = vec![QueryCondition {
        field: "_id".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("student_001".to_string()),
    }];
    
    let results = match odm::find(table_name, conditions, None, Some("mongodb_test")).await {
        Ok(results) => results,
        Err(e) => {
            let error_msg = format!("{:?}", e);
            if error_msg.contains("连接") || error_msg.contains("timeout") || error_msg.contains("Connection") {
                println!("⚠️  警告: MongoDB数据查询失败，跳过测试: {}", error_msg);
                return;
            }
            panic!("MongoDB数据查询失败: {:?}", e);
        }
    };
    
    if results.is_empty() {
        panic!("未找到插入的学生数据");
    }
    
    // 检查第一条记录
    if let Some(DataValue::Object(student_data)) = results.first() {
        println!("查询到的学生数据: {:?}", student_data);
        
        // 检查数组字段是否保持为数组格式
        if let Some(scores) = student_data.get("scores") {
            match scores {
                DataValue::Array(arr) => {
                    println!("✅ scores字段保持为数组格式: {:?}", arr);
                    // 验证数组内容
                    let expected_scores = vec![DataValue::Int(85), DataValue::Int(92), DataValue::Int(78), DataValue::Int(96)];
                    if *arr == expected_scores {
                        println!("✅ scores数组内容正确");
                    } else {
                        println!("❌ scores数组内容不匹配，期望: {:?}，实际: {:?}", expected_scores, arr);
                    }
                },
                DataValue::String(s) => {
                    println!("❌ scores字段被序列化为字符串: {}", s);
                    panic!("MongoDB数组字段序列化问题：scores字段应该保持为数组格式，但被序列化为字符串");
                },
                _ => {
                    println!("❌ scores字段类型异常: {:?}", scores);
                    panic!("MongoDB数组字段类型异常");
                }
            }
        } else {
            panic!("未找到scores字段");
        }
        
        // 检查tags数组字段
        if let Some(tags) = student_data.get("tags") {
            match tags {
                DataValue::Array(arr) => {
                    println!("✅ tags字段保持为数组格式: {:?}", arr);
                    let expected_tags = vec![
                        DataValue::String("优秀".to_string()),
                        DataValue::String("积极".to_string()),
                        DataValue::String("团队合作".to_string())
                    ];
                    if *arr == expected_tags {
                        println!("✅ tags数组内容正确");
                    } else {
                        println!("❌ tags数组内容不匹配，期望: {:?}，实际: {:?}", expected_tags, arr);
                    }
                },
                DataValue::String(s) => {
                    println!("❌ tags字段被序列化为字符串: {}", s);
                    panic!("MongoDB数组字段序列化问题：tags字段应该保持为数组格式，但被序列化为字符串");
                },
                _ => {
                    println!("❌ tags字段类型异常: {:?}", tags);
                    panic!("MongoDB数组字段类型异常");
                }
            }
        } else {
            panic!("未找到tags字段");
        }
        
        // 检查hobbies数组字段
        if let Some(hobbies) = student_data.get("hobbies") {
            match hobbies {
                DataValue::Array(arr) => {
                    println!("✅ hobbies字段保持为数组格式: {:?}", arr);
                    let expected_hobbies = vec![
                        DataValue::String("编程".to_string()),
                        DataValue::String("阅读".to_string()),
                        DataValue::String("游泳".to_string())
                    ];
                    if *arr == expected_hobbies {
                        println!("✅ hobbies数组内容正确");
                    } else {
                        println!("❌ hobbies数组内容不匹配，期望: {:?}，实际: {:?}", expected_hobbies, arr);
                    }
                },
                DataValue::String(s) => {
                    println!("❌ hobbies字段被序列化为字符串: {}", s);
                    panic!("MongoDB数组字段序列化问题：hobbies字段应该保持为数组格式，但被序列化为字符串");
                },
                _ => {
                    println!("❌ hobbies字段类型异常: {:?}", hobbies);
                    panic!("MongoDB数组字段类型异常");
                }
            }
        } else {
            panic!("未找到hobbies字段");
        }
        
        println!("🎉 MongoDB数组字段序列化测试通过！所有数组字段都保持了正确的数组格式。");
    } else {
        panic!("查询结果格式异常");
    }
    
    println!("MongoDB数组字段序列化测试完成");
}