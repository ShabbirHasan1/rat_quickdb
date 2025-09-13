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

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化库
    init(true);
    
    // 添加数据库连接
    let config = sqlite_config(":memory:")
        .alias("main")
        .pool_config(default_pool_config())
        .build();
    
    add_database(config).await?;
    
    // 获取连接
    let conn = get_connection("main").await?;
    
    // 执行查询
    let result = conn.query("SELECT 1").await?;
    
    Ok(())
}
```

### 模型定义

```rust
use rat_quickdb::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Model)]
#[model(table_name = "users")]
struct User {
    #[model(primary_key)]
    id: i64,
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
    
    // 获取模型管理器
    let user_manager = ModelManager::<User>::new("main");
    
    // 创建用户
    let user = User {
        id: 1,
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        created_at: Utc::now(),
    };
    
    user_manager.create(&user).await?;
    
    // 查询用户
    let found_user = user_manager.find_by_id(1).await?;
    println!("找到用户: {:?}", found_user);
    
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

## 核心模块

- **pool**: 连接池管理
- **manager**: 全局数据库管理器
- **odm**: 对象文档映射
- **model**: 模型定义和操作
- **adapter**: 数据库适配器
- **task_queue**: 异步任务队列
- **cache**: 缓存管理
- **id_generator**: ID生成器

## 开发状态

当前版本: 0.1.6

这是一个内部项目，主要用于统一多数据库操作接口，简化数据库操作。

## 许可证

私有项目，仅供内部使用。