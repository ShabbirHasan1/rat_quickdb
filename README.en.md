# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License](https://img.shields.io/crates/l/rat_quickdb.svg)](https://github.com/0ldm0s/rat_quickdb/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 Powerful cross-database ORM library with unified interface for SQLite, PostgreSQL, MySQL, MongoDB

## ✨ Core Features

- **🎯 Auto Index Creation**: Automatically create tables and indexes based on model definitions, no manual intervention required
- **🗄️ Multi-Database Support**: SQLite, PostgreSQL, MySQL, MongoDB
- **🔗 Unified API**: Consistent interface for different databases
- **🏊 Connection Pool Management**: Efficient connection pool and lock-free queue architecture
- **⚡ Async Support**: Based on Tokio async runtime
- **🧠 Smart Caching**: Built-in caching support (based on rat_memcache)
- **🆔 ID Generation**: Snowflake algorithm and MongoDB auto-increment ID generators
- **🐍 Python Bindings**: Optional Python API support
- **📋 Task Queue**: Built-in async task queue system
- **🔍 Type Safety**: Strong type model definitions and validation

## 📦 Installation

Add dependency in `Cargo.toml`:

```toml
[dependencies]
rat_quickdb = "0.1.7"
```

## 🚀 Quick Start

### Basic Usage

```rust
use rat_quickdb::*;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // Initialize library
    init();

    // Add SQLite database connection
    let config = sqlite_config(
        "main",
        ":memory:",
        PoolConfig::default()
    )?;
    add_database(config).await?;

    // Create user data
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String("Zhang San".to_string()));
    user_data.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));

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

## 🔧 Database Configuration

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

**Current Version**: 0.1.7

**Supported Rust Version**: 1.70+

**Important Update**: v0.1.7 adds auto index creation, LGPL-v3 license, and improved documentation!

## 📄 License

This project is licensed under the [LGPL-v3](LICENSE) license.

## 🤝 Contributing

Welcome to submit Issues and Pull Requests to improve this project!

## 📞 Contact

For questions or suggestions, please contact:
- Create Issue: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- Email: oldmos@gmail.com