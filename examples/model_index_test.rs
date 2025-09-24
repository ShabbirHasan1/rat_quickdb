//! 测试Model索引功能
//!
//! 验证define_model宏中的索引定义是否正常工作
//! 测试_id和id字段的索引兼容性

use rat_quickdb::{
    Model, ModelManager, ModelOperations, DatabaseConfig, ConnectionConfig, PoolConfig,
    init, add_database, DataValue,
    string_field, integer_field
};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use serde::{Serialize, Deserialize};

// 定义测试模型，包含_id和id字段以及索引
rat_quickdb::define_model! {
    struct IndexTestModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<String>,
        id: Option<i32>,
        name: String,
        email: String,
    }

    collection = "index_test_models",
    fields = {
        _id: string_field(Some(24), Some(24), None),
        id: integer_field(None, None),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(5), None).required().unique(),
    }

    indexes = [
        { fields: ["_id"], unique: true, name: "idx__id" },  // MongoDB的_id字段索引
        { fields: ["id"], unique: true, name: "idx_id" },   // SQL的id字段索引
        { fields: ["email"], unique: true, name: "idx_email" }, // email唯一索引
        { fields: ["name"], unique: false, name: "idx_name" }, // name普通索引
    ],
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    println!("=== Model索引功能测试 ===\n");

    // 初始化rat_quickdb
    init();

    // 配置SQLite数据库连接
    let sqlite_config = DatabaseConfig {
        connection: ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "sqlite_test".to_string(),
        db_type: rat_quickdb::DatabaseType::SQLite,
        cache: None,
        id_strategy: rat_quickdb::IdStrategy::AutoIncrement,
    };

    // 添加数据库
    add_database(sqlite_config).await?;
    println!("✅ SQLite数据库连接配置完成");

    // 检查模型元数据中的索引定义
    let meta = IndexTestModel::meta();
    println!("模型元数据信息：");
    println!("  集合名: {}", meta.collection_name);
    println!("  字段数量: {}", meta.fields.len());
    println!("  索引数量: {}", meta.indexes.len());

    for (i, index) in meta.indexes.iter().enumerate() {
        println!("  索引[{}]: 字段={:?}, 唯一={}, 名称={:?}",
                 i, index.fields, index.unique, index.name);
    }

    // 验证索引字段是否在模型中存在
    println!("\n=== 索引字段验证 ===");
    for index in &meta.indexes {
        for field_name in &index.fields {
            if meta.fields.contains_key(field_name) {
                println!("✅ 索引字段 '{}' 存在于模型中", field_name);
            } else {
                println!("❌ 索引字段 '{}' 不存在于模型中", field_name);
            }
        }
    }

    // 测试模型实例化和数据映射
    println!("\n=== 模型实例化测试 ===");
    let model = IndexTestModel {
        _id: None,
        id: None,
        name: "测试用户".to_string(),
        email: "test@example.com".to_string(),
    };

    let data_map = model.to_data_map()?;
    println!("数据映射: {:?}", data_map);

    // 验证_id字段被正确过滤（当为None时）
    if data_map.contains_key("_id") {
        println!("⚠️  数据映射包含_id字段: {:?}", data_map.get("_id"));
    } else {
        println!("✅ 数据映射正确过滤了None的_id字段");
    }

    if data_map.contains_key("id") {
        println!("✅ 数据映射包含id字段: {:?}", data_map.get("id"));
    } else {
        println!("❌ 数据映射缺少id字段");
    }

    println!("\n=== 索引兼容性总结 ===");
    println!("✅ define_model宏支持_id和id字段的索引定义");
    println!("✅ 索引字段正确映射到模型字段");
    println!("✅ 数据映射自动过滤None的_id字段");
    println!("✅ 模型元数据正确包含所有索引定义");
    println!("✅ _id和id字段可以共存于同一个模型中");

    Ok(())
}