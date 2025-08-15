//! 测试自动建表功能
//! 
//! 这个测试程序验证在插入数据时是否能自动创建表

use rat_quickdb::{
    config::{sqlite_config, default_pool_config, GlobalConfig, GlobalConfigBuilder, AppConfigBuilder, LoggingConfigBuilder, Environment, LogLevel},
    types::{DataValue},
    odm::{get_odm_manager, OdmOperations},
    manager::add_database,
    error::QuickDbResult,
    init,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio;
use zerg_creep::{info, error};

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    init();
    
    info!("开始测试自动建表功能...");
    
    // 创建连接池配置
    let pool_config = default_pool_config(1, 5)?;
    
    // 创建SQLite数据库配置
    let db_config = sqlite_config("test_db", ":memory:", pool_config)?;
    
    // 添加数据库配置到管理器
    add_database(db_config).await?;
    
    info!("数据库连接成功！");
    
    // 获取ODM管理器
    let odm = get_odm_manager().await;
    
    // 准备测试数据 - 用户表
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String("张三".to_string()));
    user_data.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));
    user_data.insert("age".to_string(), DataValue::Int(25));
    user_data.insert("is_active".to_string(), DataValue::Bool(true));
    
    info!("插入用户数据: {:?}", user_data);
    
    // 插入数据 - 这里应该会自动创建表
    let user_id = odm.create("users", user_data, Some("test_db")).await?;
    info!("用户创建成功，ID: {}", user_id);
    
    // 准备第二条测试数据
    let mut user_data2 = HashMap::new();
    user_data2.insert("name".to_string(), DataValue::String("李四".to_string()));
    user_data2.insert("email".to_string(), DataValue::String("lisi@example.com".to_string()));
    user_data2.insert("age".to_string(), DataValue::Int(30));
    user_data2.insert("is_active".to_string(), DataValue::Bool(false));
    
    info!("插入第二个用户数据: {:?}", user_data2);
    
    // 插入第二条数据
    let user_id2 = odm.create("users", user_data2, Some("test_db")).await?;
    info!("第二个用户创建成功，ID: {}", user_id2);
    
    info!("自动建表功能测试完成！!");
    
    Ok(())
}