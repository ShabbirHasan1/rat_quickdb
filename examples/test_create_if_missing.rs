//! 测试 create_if_missing 参数功能
//! 验证数据库文件是否真正被创建

use rat_quickdb::{
    types::*,
    manager::{PoolManager, get_global_pool_manager},
    error::QuickDbResult,
};
use std::path::Path;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🧪 测试 create_if_missing 参数功能");
    println!("======================================\n");
    
    let test_db_path = "./test_data/create_if_missing_test.db";
    
    // 确保测试文件不存在
    if Path::new(test_db_path).exists() {
        tokio::fs::remove_file(test_db_path).await.ok();
        println!("🗑️  清理已存在的测试文件");
    }
    
    // 确保测试目录不存在
    if Path::new("./test_data").exists() {
        tokio::fs::remove_dir_all("./test_data").await.ok();
        println!("🗑️  清理已存在的测试目录");
    }
    
    println!("📁 测试前状态:");
    println!("   - 数据库文件存在: {}", Path::new(test_db_path).exists());
    println!("   - 测试目录存在: {}\n", Path::new("./test_data").exists());
    
    // 测试 create_if_missing = true
    println!("🔧 测试 create_if_missing = true...");
    let config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: test_db_path.to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "test_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };
    
    // 初始化数据库连接
    let pool_manager = get_global_pool_manager();
    match pool_manager.add_database(config).await {
        Ok(_) => {
            println!("✅ 数据库连接创建成功");
            
            // 检查文件是否被创建
            println!("\n📁 测试后状态:");
            println!("   - 数据库文件存在: {}", Path::new(test_db_path).exists());
            println!("   - 测试目录存在: {}", Path::new("./test_data").exists());
            
            if Path::new(test_db_path).exists() {
                println!("\n🎉 create_if_missing 功能正常工作！数据库文件已创建");
                
                // 获取文件大小
                if let Ok(metadata) = tokio::fs::metadata(test_db_path).await {
                    println!("📊 数据库文件大小: {} 字节", metadata.len());
                }
            } else {
                println!("\n❌ create_if_missing 功能异常！数据库文件未创建");
            }
        }
        Err(e) => {
            println!("❌ 数据库连接创建失败: {}", e);
        }
    }
    
    // 清理测试文件
    println!("\n🧹 清理测试文件...");
    if Path::new(test_db_path).exists() {
        tokio::fs::remove_file(test_db_path).await.ok();
        println!("🗑️  已删除: {}", test_db_path);
    }
    
    if Path::new("./test_data").exists() {
        tokio::fs::remove_dir("./test_data").await.ok();
        println!("🗑️  已删除: ./test_data");
    }
    
    println!("\n✅ 测试完成！");
    Ok(())
}