# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License](https://img.shields.io/crates/l/rat_quickdb.svg)](https://github.com/0ldm0s/rat_quickdb/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 强大的跨数据库ORM库，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

## ✨ 核心特性

- **🎯 自动索引创建**: 基于模型定义自动创建表和索引，无需手动干预
- **🗄️ 多数据库支持**: SQLite、PostgreSQL、MySQL、MongoDB
- **🔗 统一API**: 一致的接口操作不同数据库
- **🏊 连接池管理**: 高效的连接池和无锁队列架构
- **⚡ 异步支持**: 基于Tokio的异步运行时
- **🧠 智能缓存**: 内置缓存支持（基于rat_memcache）
- **🆔 ID生成**: 雪花算法和MongoDB自增ID生成器
- **🐍 Python绑定**: 可选Python API支持
- **📋 任务队列**: 内置异步任务队列系统
- **🔍 类型安全**: 强类型模型定义和验证

## 📦 安装

在`Cargo.toml`中添加依赖：

```toml
[dependencies]
rat_quickdb = "0.1.7"
```

## 🚀 快速开始

### 基本使用

```rust
use rat_quickdb::*;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化库
    init();

    // 添加SQLite数据库连接
    let config = sqlite_config(
        "main",
        ":memory:",
        PoolConfig::default()
    )?;
    add_database(config).await?;

    // 创建用户数据
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String("张三".to_string()));
    user_data.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));

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

## 🔧 数据库配置

### SQLite
```rust
use rat_quickdb::*;

let pool_config = PoolConfig::builder()
    .max_connections(10)
    .min_connections(2)
    .connection_timeout(30)
    .idle_timeout(300)
    .build()?;

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
let tls_config = TlsConfig {
    enabled: true,
    verify_server_cert: false,
    verify_hostname: false,
    ..Default::default()
};

let config = DatabaseConfig {
    db_type: DatabaseType::PostgreSQL,
    connection: ConnectionConfig::PostgreSQL {
        host: "localhost".to_string(),
        port: 5432,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
        ssl_mode: Some("prefer".to_string()),
        tls_config: Some(tls_config),
    },
    pool: pool_config,
    alias: "postgres_db".to_string(),
    cache: None,
    id_strategy: IdStrategy::AutoIncrement,
};
add_database(config).await?;
```

### MySQL
```rust
use rat_quickdb::*;
use std::collections::HashMap;

let pool_config = PoolConfig::default();
let tls_config = TlsConfig {
    enabled: true,
    verify_server_cert: false,
    verify_hostname: false,
    ..Default::default()
};

let mut ssl_opts = HashMap::new();
ssl_opts.insert("ssl_mode".to_string(), "PREFERRED".to_string());

let config = DatabaseConfig {
    db_type: DatabaseType::MySQL,
    connection: ConnectionConfig::MySQL {
        host: "localhost".to_string(),
        port: 3306,
        database: "mydatabase".to_string(),
        username: "username".to_string(),
        password: "password".to_string(),
        ssl_opts: Some(ssl_opts),
        tls_config: Some(tls_config),
    },
    pool: pool_config,
    alias: "mysql_db".to_string(),
    cache: None,
    id_strategy: IdStrategy::AutoIncrement,
};
add_database(config).await?;
```

### MongoDB
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

**当前版本**: 0.1.7

**支持Rust版本**: 1.70+

**重要更新**: v0.1.7 添加了自动索引创建功能、LGPL-v3许可证和改进的文档！

## 📄 许可证

本项目采用 [LGPL-v3](LICENSE) 许可证。

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个项目！

## 📞 联系方式

如有问题或建议，请通过以下方式联系：
- 创建Issue: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- 邮箱: oldmos@gmail.com