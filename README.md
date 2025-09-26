# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License: LGPL-3.0](https://img.shields.io/badge/License-LGPL--3.0-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 强大的跨数据库ODM库，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

**🌐 语言版本**: [中文](README.md) | [English](README.en.md) | [日本語](README.ja.md)

## ✨ 核心特性

- **🎯 自动索引创建**: 基于模型定义自动创建表和索引，无需手动干预
- **🗄️ 多数据库支持**: SQLite、PostgreSQL、MySQL、MongoDB
- **🔗 统一API**: 一致的接口操作不同数据库
- **🏊 连接池管理**: 高效的连接池和无锁队列架构
- **⚡ 异步支持**: 基于Tokio的异步运行时
- **🧠 智能缓存**: 内置缓存支持（基于rat_memcache），支持TTL过期和回退机制
- **🆔 多种ID生成策略**: AutoIncrement、UUID、Snowflake、ObjectId、Custom前缀
- **📝 日志控制**: 由调用者完全控制日志初始化，避免库自动初始化冲突
- **🐍 Python绑定**: 可选Python API支持
- **📋 任务队列**: 内置异步任务队列系统
- **🔍 类型安全**: 强类型模型定义和验证

## 📦 安装

在`Cargo.toml`中添加依赖：

```toml
[dependencies]
rat_quickdb = "0.1.8"
```

## 🚀 快速开始

### 基本使用

```rust
use rat_quickdb::*;
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig, CompressionConfig, CompressionAlgorithm};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化库（日志系统由调用者自行初始化）
    init();

    // 添加SQLite数据库连接（带缓存配置）
    let config = DatabaseConfig::builder()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .pool(PoolConfig::default())
        .alias("main".to_string())
        .id_strategy(IdStrategy::AutoIncrement)
        .cache(CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            ttl_config: TtlConfig {
                default_ttl_secs: 300,
                max_ttl_secs: 3600,
                check_interval_secs: 60,
            },
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 64,
                enable_stats: true,
            },
            l2_config: None,
            compression_config: CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::Lz4,
                threshold_bytes: 1024,
            },
            version: "1".to_string(),
        })
        .build()?;
    add_database(config).await?;

    // 创建用户数据
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String("张三".to_string()));
    user_data.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));
    user_data.insert("age".to_string(), DataValue::Int(25));

    // 创建用户记录
    create("users", user_data, Some("main")).await?;

    // 查询用户
    let user = find_by_id("users", "1", Some("main")).await?;
    println!("找到用户: {:?}", user);

    Ok(())
}
```

### 模型定义（推荐方式）

```rust
use rat_quickdb::*;
use serde::{Serialize, Deserialize};

// 定义用户模型
rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        username: String,
        email: String,
        age: i32,
        is_active: bool,
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        email: string_field(Some(255), Some(5), None).required().unique(),
        age: integer_field(Some(0), Some(150)),
        is_active: boolean_field(),
    }

    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["age"], unique: false, name: "idx_age" },
    ],
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    init();

    // 添加数据库
    let config = sqlite_config("main", "test.db", PoolConfig::default())?;
    add_database(config).await?;

    // 创建用户（自动创建表和索引）
    let user = User {
        id: None,
        username: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        age: 25,
        is_active: true,
    };

    // 保存用户（自动处理所有数据库操作）
    let user_id = user.save().await?;
    println!("用户创建成功，ID: {}", user_id);

    // 查询用户
    if let Some(found_user) = ModelManager::<User>::find_by_id(&user_id).await? {
        println!("找到用户: {} ({})", found_user.username, found_user.email);
    }

    Ok(())
}
```

## 🆔 ID生成策略

rat_quickdb支持多种ID生成策略，满足不同场景的需求：

### AutoIncrement（自增ID）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?
```

### UUID（通用唯一标识符）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Uuid)
    .build()?
```

### Snowflake（雪花算法）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Snowflake {
        machine_id: 1,
        datacenter_id: 1
    })
    .build()?
```

### ObjectId（MongoDB风格）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::ObjectId)
    .build()?
```

### Custom（自定义前缀）
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Custom("user_".to_string()))
    .build()?
```

## 🧠 缓存配置

### 基本缓存配置
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig};

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 300,  // 5分钟缓存
        max_ttl_secs: 3600,     // 最大1小时
        check_interval_secs: 60, // 检查间隔
    },
    l1_config: L1CacheConfig {
        max_capacity: 1000,     // 最多1000个条目
        max_memory_mb: 64,       // 64MB内存限制
        enable_stats: true,      // 启用统计
    },
    l2_config: None,           // 不使用L2磁盘缓存
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

### 缓存统计和管理
```rust
// 获取缓存统计信息
let stats = get_cache_stats("default").await?;
println!("缓存命中率: {:.2}%", stats.hit_rate * 100.0);
println!("缓存条目数: {}", stats.entries);

// 清理缓存
clear_cache("default").await?;
clear_all_caches().await?;
```

## 📝 日志控制

rat_quickdb现在完全由调用者控制日志初始化：

```rust
use rat_logger::{Logger, LoggerBuilder, LevelFilter};

// 调用者负责初始化日志系统
let logger = LoggerBuilder::new()
    .with_level(LevelFilter::Debug)
    .with_file("app.log")
    .build();

logger.init().expect("日志初始化失败");

// 然后初始化rat_quickdb（不再自动初始化日志）
rat_quickdb::init();
```

## 🔧 数据库配置

### SQLite
```rust
use rat_quickdb::*;

let pool_config = PoolConfig::default();

let config = sqlite_config(
    "sqlite_db",
    "./test.db",
    pool_config
)?;
add_database(config).await?;
```

### PostgreSQL
```rust
use rat_quickdb::*;

let pool_config = PoolConfig::default();
let config = postgres_config(
    "postgres_db",
    "localhost",
    5432,
    "mydatabase",
    "username",
    "password",
    pool_config
)?;
add_database(config).await?;
```

### MySQL
```rust
use rat_quickdb::*;

let pool_config = PoolConfig::default();
let config = mysql_config(
    "mysql_db",
    "localhost",
    3306,
    "mydatabase",
    "username",
    "password",
    pool_config
)?;
add_database(config).await?;
```

### MongoDB

#### 基础配置（使用便捷函数）
```rust
use rat_quickdb::*;

let pool_config = PoolConfig::default();
let config = mongodb_config(
    "mongodb_db",
    "localhost",
    27017,
    "mydatabase",
    Some("username"),
    Some("password"),
    pool_config
)?;
add_database(config).await?;
```

#### 高级配置（使用构建器）
```rust
use rat_quickdb::*;

let pool_config = PoolConfig::default();
let tls_config = TlsConfig {
    enabled: true,
    verify_server_cert: false,
    verify_hostname: false,
    ..Default::default()
};

let zstd_config = ZstdConfig {
    enabled: true,
    compression_level: Some(3),
    compression_threshold: Some(1024),
};

let builder = MongoDbConnectionBuilder::new("localhost", 27017, "mydatabase")
    .with_auth("username", "password")
    .with_auth_source("admin")
    .with_direct_connection(true)
    .with_tls_config(tls_config)
    .with_zstd_config(zstd_config);

let config = mongodb_config_with_builder(
    "mongodb_db",
    builder,
    pool_config,
)?;
add_database(config).await?;
```

## 🛠️ 核心API

### 数据库管理
- `init()` - 初始化库
- `add_database(config)` - 添加数据库配置
- `remove_database(alias)` - 移除数据库配置
- `get_aliases()` - 获取所有数据库别名
- `set_default_alias(alias)` - 设置默认数据库别名

### 模型操作（推荐）
```rust
// 保存记录
let user_id = user.save().await?;

// 查询记录
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
let users = ModelManager::<User>::find(conditions, options).await?;

// 更新记录
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("新名字".to_string()));
let updated = user.update(updates).await?;

// 删除记录
let deleted = user.delete().await?;
```

### ODM操作（底层接口）
- `create(collection, data, alias)` - 创建记录
- `find_by_id(collection, id, alias)` - 根据ID查找
- `find(collection, conditions, options, alias)` - 查询记录
- `update(collection, id, data, alias)` - 更新记录
- `delete(collection, id, alias)` - 删除记录
- `count(collection, query, alias)` - 计数
- `exists(collection, query, alias)` - 检查是否存在

## 🏗️ 架构特点

rat_quickdb采用现代化架构设计：

1. **无锁队列架构**: 避免直接持有数据库连接的生命周期问题
2. **模型自动注册**: 首次使用时自动注册模型元数据
3. **自动索引管理**: 根据模型定义自动创建表和索引
4. **跨数据库适配**: 统一的接口支持多种数据库类型
5. **异步消息处理**: 基于Tokio的高效异步处理

## 🔄 工作流程

```
应用层 → 模型操作 → ODM层 → 消息队列 → 连接池 → 数据库
    ↑                                        ↓
    └────────────── 结果返回 ←────────────────┘
```

## 📊 性能特性

- **连接池管理**: 智能连接复用和管理
- **异步操作**: 非阻塞的数据库操作
- **批量处理**: 支持批量操作优化
- **缓存集成**: 内置缓存减少数据库访问
- **压缩支持**: MongoDB支持ZSTD压缩

## 🎯 支持的字段类型

- `integer_field` - 整数字段（支持范围和约束）
- `string_field` - 字符串字段（支持长度限制，可设置大长度作为长文本使用）
- `float_field` - 浮点数字段（支持范围和精度）
- `boolean_field` - 布尔字段
- `datetime_field` - 日期时间字段
- `uuid_field` - UUID字段
- `json_field` - JSON字段
- `array_field` - 数组字段
- `list_field` - 列表字段（array_field的别名）
- `dict_field` - 字典/对象字段（基于Object类型）
- `reference_field` - 引用字段（外键）

## 📝 索引支持

- **唯一索引**: `unique()` 约束
- **复合索引**: 多字段组合索引
- **普通索引**: 基础查询优化索引
- **自动创建**: 基于模型定义自动创建
- **跨数据库**: 支持所有数据库类型的索引

## 🌟 版本信息

**当前版本**: 0.1.9

**支持Rust版本**: 1.70+

**重要更新**: v0.1.9 修正项目定位为ODM，完善了跨数据库支持！

## 📄 许可证

本项目采用 [LGPL-v3](LICENSE) 许可证。

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个项目！

## 📞 联系方式

如有问题或建议，请通过以下方式联系：
- 创建Issue: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- 邮箱: oldmos@gmail.com