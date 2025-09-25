# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License](https://img.shields.io/crates/l/rat_quickdb.svg)](https://github.com/0ldm0s/rat_quickdb/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 Powerful cross-database ORM library with unified interface for SQLite, PostgreSQL, MySQL, MongoDB

**🌐 Language Versions**: [中文](README.md) | [English](README.en.md) | [日本語](README.ja.md)

## ✨ Core Features

- **🎯 Auto Index Creation**: Automatically create tables and indexes based on model definitions, no manual intervention required
- **🗄️ Multi-Database Support**: SQLite, PostgreSQL, MySQL, MongoDB
- **🔗 Unified API**: Consistent interface for different databases
- **🏊 Connection Pool Management**: Efficient connection pool and lock-free queue architecture
- **⚡ Async Support**: Based on Tokio async runtime
- **🧠 Smart Caching**: Built-in caching support (based on rat_memcache), with TTL expiration and fallback mechanism
- **🆔 Multiple ID Generation Strategies**: AutoIncrement, UUID, Snowflake, ObjectId, Custom prefix
- **📝 Logging Control**: Complete logging initialization control by caller, avoiding library auto-initialization conflicts
- **🐍 Python Bindings**: Optional Python API support
- **📋 Task Queue**: Built-in async task queue system
- **🔍 Type Safety**: Strong type model definitions and validation

## 📦 Installation

Add dependency in `Cargo.toml`:

```toml
[dependencies]
rat_quickdb = "0.1.8"
```

## 🚀 Quick Start

### Basic Usage

```rust
use rat_quickdb::*;
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig, CompressionConfig, CompressionAlgorithm};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // Initialize library (logging system to be initialized by caller)
    init();

    // Add SQLite database connection (with cache configuration)
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

    // Create user data
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String("Zhang San".to_string()));
    user_data.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));
    user_data.insert("age".to_string(), DataValue::Int(25));

    // Create user record
    create("users", user_data, Some("main")).await?;

    // Query user
    let user = find_by_id("users", "1", Some("main")).await?;
    println!("Found user: {:?}", user);

    Ok(())
}
```

### Model Definition (Recommended)

```rust
use rat_quickdb::*;
use serde::{Serialize, Deserialize};

// Define user model
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

    // Add database
    let config = sqlite_config("main", "test.db", PoolConfig::default())?;
    add_database(config).await?;

    // Create user (automatically creates tables and indexes)
    let user = User {
        id: None,
        username: "zhangsan".to_string(),
        email: "zhangsan@example.com".to_string(),
        age: 25,
        is_active: true,
    };

    // Save user (automatically handles all database operations)
    let user_id = user.save().await?;
    println!("User created successfully, ID: {}", user_id);

    // Query user
    if let Some(found_user) = ModelManager::<User>::find_by_id(&user_id).await? {
        println!("Found user: {} ({})", found_user.username, found_user.email);
    }

    Ok(())
}
```

## 🆔 ID Generation Strategies

rat_quickdb supports multiple ID generation strategies for different use cases:

### AutoIncrement (Auto-increment ID)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::AutoIncrement)
    .build()?
```

### UUID (Universally Unique Identifier)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Uuid)
    .build()?
```

### Snowflake (Snowflake Algorithm)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Snowflake {
        machine_id: 1,
        datacenter_id: 1
    })
    .build()?
```

### ObjectId (MongoDB Style)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::ObjectId)
    .build()?
```

### Custom (Custom Prefix)
```rust
DatabaseConfig::builder()
    .id_strategy(IdStrategy::Custom("user_".to_string()))
    .build()?
```

## 🧠 Cache Configuration

### Basic Cache Configuration
```rust
use rat_quickdb::types::{CacheConfig, CacheStrategy, TtlConfig, L1CacheConfig};

let cache_config = CacheConfig {
    enabled: true,
    strategy: CacheStrategy::Lru,
    ttl_config: TtlConfig {
        default_ttl_secs: 300,  // 5 minutes cache
        max_ttl_secs: 3600,     // maximum 1 hour
        check_interval_secs: 60, // check interval
    },
    l1_config: L1CacheConfig {
        max_capacity: 1000,     // maximum 1000 entries
        max_memory_mb: 64,       // 64MB memory limit
        enable_stats: true,      // enable statistics
    },
    l2_config: None,           // no L2 disk cache
    compression_config: CompressionConfig::default(),
    version: "1".to_string(),
};

DatabaseConfig::builder()
    .cache(cache_config)
    .build()?
```

### Cache Statistics and Management
```rust
// Get cache statistics
let stats = get_cache_stats("default").await?;
println!("Cache hit rate: {:.2}%", stats.hit_rate * 100.0);
println!("Cache entries: {}", stats.entries);

// Clear cache
clear_cache("default").await?;
clear_all_caches().await?;
```

## 📝 Logging Control

rat_quickdb now gives complete logging initialization control to the caller:

```rust
use rat_logger::{Logger, LoggerBuilder, LevelFilter};

// Caller is responsible for initializing the logging system
let logger = LoggerBuilder::new()
    .with_level(LevelFilter::Debug)
    .with_file("app.log")
    .build();

logger.init().expect("Failed to initialize logging");

// Then initialize rat_quickdb (no longer auto-initializes logging)
rat_quickdb::init();
```

## 🔧 Database Configuration

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

#### Basic Configuration (using convenience function)
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

#### Advanced Configuration (using builder)
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

## 🛠️ Core API

### Database Management
- `init()` - Initialize library
- `add_database(config)` - Add database configuration
- `remove_database(alias)` - Remove database configuration
- `get_aliases()` - Get all database aliases
- `set_default_alias(alias)` - Set default database alias

### Model Operations (Recommended)
```rust
// Save record
let user_id = user.save().await?;

// Query record
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
let users = ModelManager::<User>::find(conditions, options).await?;

// Update record
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("new_name".to_string()));
let updated = user.update(updates).await?;

// Delete record
let deleted = user.delete().await?;
```

### ODM Operations (Low-level)
- `create(collection, data, alias)` - Create record
- `find_by_id(collection, id, alias)` - Find by ID
- `find(collection, conditions, options, alias)` - Query records
- `update(collection, id, data, alias)` - Update record
- `delete(collection, id, alias)` - Delete record
- `count(collection, query, alias)` - Count records
- `exists(collection, query, alias)` - Check existence

## 🏗️ Architecture Features

rat_quickdb adopts modern architecture design:

1. **Lock-free Queue Architecture**: Avoids direct database connection lifecycle issues
2. **Model Auto-registration**: Automatically registers model metadata on first use
3. **Auto Index Management**: Automatically creates tables and indexes based on model definitions
4. **Cross-database Adapter**: Unified interface supporting multiple database types
5. **Async Message Processing**: Efficient async processing based on Tokio

## 🔄 Workflow

```
Application Layer → Model Operations → ODM Layer → Message Queue → Connection Pool → Database
    ↑                                                             ↓
    └───────────────── Result Return ←───────────────────────────┘
```

## 📊 Performance Features

- **Connection Pool Management**: Intelligent connection reuse and management
- **Async Operations**: Non-blocking database operations
- **Batch Processing**: Supports batch operation optimization
- **Cache Integration**: Built-in caching reduces database access
- **Compression Support**: MongoDB supports ZSTD compression

## 🎯 Supported Field Types

- `integer_field` - Integer fields (with range and constraints)
- `string_field` - String fields (with length limits, can use large length as text replacement)
- `float_field` - Floating-point number fields (with range and precision)
- `boolean_field` - Boolean fields
- `datetime_field` - Date-time fields
- `uuid_field` - UUID fields
- `json_field` - JSON fields
- `array_field` - Array fields
- `list_field` - List fields (array_field alias)
- `dict_field` - Dictionary/Object fields (object_field replacement)
- `reference_field` - Reference fields (foreign keys)

## 📝 Index Support

- **Unique Indexes**: `unique()` constraints
- **Composite Indexes**: Multi-field combination indexes
- **Regular Indexes**: Basic query optimization indexes
- **Auto Creation**: Automatically created based on model definitions
- **Cross-database**: Supports all database index types

## 🌟 Version Information

**Current Version**: 0.1.8

**Supported Rust Version**: 1.70+

**Important Update**: v0.1.8 enhances ID generation strategies, cache configuration, and logging control, with all core features validated!

## 📄 License

This project is licensed under the [LGPL-v3](LICENSE) license.

## 🤝 Contributing

Welcome to submit Issues and Pull Requests to improve this project!

## 📞 Contact

For questions or suggestions, please contact:
- Create Issue: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- Email: oldmos@gmail.com