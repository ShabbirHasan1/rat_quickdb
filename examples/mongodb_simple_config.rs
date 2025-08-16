//! MongoDB 简单配置示例
//! 
//! 本示例展示如何使用新的分离字段配置方式连接 MongoDB，
//! 避免手动进行 URL 编码，提供更好的用户体验。

use rat_quickdb::{
    config::{mongodb_config, mongodb_config_with_builder},
    error::QuickDbError,
    types::{DatabaseConfig, MongoDbConnectionBuilder, PoolConfig, TlsConfig, ZstdConfig},
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), QuickDbError> {
    // 初始化日志
    env_logger::init();

    println!("MongoDB 连接配置示例");
    println!("====================");

    // 示例1: 使用便捷函数配置 MongoDB（基本配置）
    println!("\n1. 基本配置示例:");
    let basic_config = basic_mongodb_config()?;
    println!("   ✓ 基本配置创建成功: {}", basic_config.alias);

    // 示例2: 使用便捷函数配置 MongoDB（带认证）
    println!("\n2. 带认证配置示例:");
    let auth_config = authenticated_mongodb_config()?;
    println!("   ✓ 认证配置创建成功: {}", auth_config.alias);

    // 示例3: 使用构建器模式配置 MongoDB（完整配置）
    println!("\n3. 完整配置示例（使用构建器）:");
    let full_config = full_mongodb_config_with_builder()?;
    println!("   ✓ 完整配置创建成功: {}", full_config.alias);

    // 示例4: 展示密码中包含特殊字符的情况
    println!("\n4. 特殊字符密码示例:");
    let special_char_config = special_character_password_config()?;
    println!("   ✓ 特殊字符密码配置创建成功: {}", special_char_config.alias);
    println!("   ✓ 密码自动进行了 URL 编码，无需手动处理");

    println!("\n所有配置示例创建成功！");
    Ok(())
}

/// 基本 MongoDB 配置（无认证）
fn basic_mongodb_config() -> Result<DatabaseConfig, QuickDbError> {
    let pool_config = PoolConfig::builder()
        .max_connections(5)
        .min_connections(1)
        .connection_timeout(10)
        .idle_timeout(300)
        .max_lifetime(600)
        .build()?;

    mongodb_config(
        "basic_mongodb",
        "localhost",
        27017,
        "testdb",
        None::<&str>, // 无用户名
        None::<&str>, // 无密码
        pool_config,
    )
}

/// 带认证的 MongoDB 配置
fn authenticated_mongodb_config() -> Result<DatabaseConfig, QuickDbError> {
    let pool_config = PoolConfig::builder()
        .max_connections(10)
        .min_connections(2)
        .connection_timeout(15)
        .idle_timeout(600)
        .max_lifetime(1800)
        .build()?;

    mongodb_config(
        "auth_mongodb",
        "localhost",
        27017,
        "myapp",
        Some("myuser"),
        Some("mypassword"),
        pool_config,
    )
}

/// 使用构建器模式的完整 MongoDB 配置
fn full_mongodb_config_with_builder() -> Result<DatabaseConfig, QuickDbError> {
    let pool_config = PoolConfig::builder()
        .max_connections(20)
        .min_connections(5)
        .connection_timeout(30)
        .idle_timeout(600)
        .max_lifetime(1800)
        .build()?;

    // 配置 TLS
    let tls_config = TlsConfig {
        enabled: true,
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
        verify_server_cert: false,
        verify_hostname: false,
        min_tls_version: None,
        cipher_suites: None,
    };

    // 配置 ZSTD 压缩
    let zstd_config = ZstdConfig {
        enabled: true,
        compression_level: Some(3),
        compression_threshold: Some(1024),
    };

    // 使用构建器创建连接配置
    let builder = MongoDbConnectionBuilder::new("db.example.com", 27017, "production")
        .with_auth("admin", "secure_password")
        .with_auth_source("admin")
        .with_direct_connection(true)
        .with_tls_config(tls_config)
        .with_zstd_config(zstd_config)
        .with_option("retryWrites", "true")
        .with_option("w", "majority");

    mongodb_config_with_builder(
        "full_mongodb",
        builder,
        pool_config,
    )
}

/// 包含特殊字符密码的配置示例
fn special_character_password_config() -> Result<DatabaseConfig, QuickDbError> {
    let pool_config = PoolConfig::builder()
        .max_connections(10)
        .min_connections(2)
        .connection_timeout(30)
        .idle_timeout(600)
        .max_lifetime(1800)
        .build()?;

    // 密码包含各种特殊字符，无需手动编码
    let complex_password = "P@ssw0rd!@#$%^&*()_+-=[]{}|\\:;\"'<>,.?/`~";

    let builder = MongoDbConnectionBuilder::new("localhost", 27017, "testdb")
        .with_auth("testuser", complex_password)
        .with_auth_source("testdb")
        .with_direct_connection(true)
        .with_option("authMechanism", "SCRAM-SHA-256");

    mongodb_config_with_builder(
        "special_char_mongodb",
        builder,
        pool_config,
    )
}