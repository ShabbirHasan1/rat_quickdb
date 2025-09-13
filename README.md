# rat_quickdb

跨数据库ORM库，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

## 特性

- **多数据库支持**: SQLite、PostgreSQL、MySQL、MongoDB
- **统一API**: 一致的接口操作不同数据库
- **连接池管理**: 高效的连接池和无锁队列
- **异步支持**: 基于Tokio的异步运行时
- **缓存集成**: 内置缓存支持（基于rat_memcache）
- **ID生成**: 雪花算法和MongoDB自增ID生成器
- **Python绑定**: 可选Python API支持
- **任务队列**: 内置异步任务队列系统

## 安装

在`Cargo.toml`中添加依赖：

```toml
[dependencies]
rat_quickdb = "0.1.6"
```

## 快速开始

### 基本使用

```rust
use rat_quickdb::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化库
    init(true);

    // 添加SQLite数据库连接
    let config = sqlite_config(":memory:")
        .alias("main")
        .pool_config(default_pool_config())
        .build();

    add_database(config).await?;

    // 创建用户表
    let mut user_data = HashMap::new();
    user_data.insert("id".to_string(), DataValue::String("1".to_string()));
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

### 模型定义和使用

```rust
use rat_quickdb::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Model)]
#[model(table_name = "users")]
struct User {
    #[model(primary_key)]
    id: String,
    name: String,
    email: String,
    created_at: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    init(true);

    // 添加数据库
    let config = sqlite_config("test.db")
        .alias("main")
        .build();
    add_database(config).await?;

    // 创建用户
    let user = User {
        id: "1".to_string(),
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        created_at: Utc::now(),
    };

    // 序列化并保存用户
    let user_data = user.to_data_map()?;
    create("users", user_data, Some("main")).await?;

    // 查询用户
    let found_user = find_by_id("users", "1", Some("main")).await?;
    if let Some(user_data) = found_user {
        let user: User = User::from_data_value(user_data)?;
        println!("找到用户: {:?}", user);
    }

    Ok(())
}
```

## 数据库配置

### SQLite
```rust
let config = sqlite_config("test.db")
    .alias("sqlite_db")
    .pool_config(default_pool_config())
    .build();
```

### PostgreSQL
```rust
let config = postgres_config("postgres://user:pass@localhost/db")
    .alias("pg_db")
    .pool_config(default_pool_config())
    .build();
```

### MySQL
```rust
let config = mysql_config("mysql://user:pass@localhost/db")
    .alias("mysql_db")
    .pool_config(default_pool_config())
    .build();
```

### MongoDB
```rust
let config = mongodb_config("mongodb://localhost:27017")
    .alias("mongo_db")
    .database("mydb")
    .pool_config(default_pool_config())
    .build();
```

## 核心API

### 数据库管理
- `add_database(config)` - 添加数据库配置
- `remove_database(alias)` - 移除数据库配置
- `get_aliases()` - 获取所有数据库别名
- `set_default_alias(alias)` - 设置默认数据库别名

### ODM操作（主要接口）
- `create(collection, data, alias)` - 创建记录
- `find_by_id(collection, id, alias)` - 根据ID查找
- `find(collection, conditions, options, alias)` - 查询记录
- `update(collection, id, data, alias)` - 更新记录
- `delete(collection, id, alias)` - 删除记录
- `count(collection, query, alias)` - 计数
- `exists(collection, query, alias)` - 检查是否存在

### 模型特征
所有模型需要实现 `Model` trait，提供：
- `meta()` - 返回模型元数据
- `collection_name()` - 集合/表名
- `database_alias()` - 数据库别名
- `to_data_map()` - 序列化为数据映射
- `from_data_value()` - 从数据值反序列化

## 架构说明

rat_quickdb采用无锁队列架构：
1. **应用层**调用ODM函数（create/find/update等）
2. **ODM层**将操作封装为消息发送到队列
3. **连接池工作线程**处理消息并执行实际数据库操作
4. **结果**通过oneshot通道返回给调用方

这种设计避免了直接持有数据库连接的生命周期问题。

## 开发状态

当前版本: 0.1.6

这是一个内部项目，主要用于统一多数据库操作接口，简化数据库操作。

## 许可证

私有项目，仅供内部使用。