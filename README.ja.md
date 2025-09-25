# rat_quickdb

[![Crates.io](https://img.shields.io/crates/v/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)
[![Documentation](https://docs.rs/rat_quickdb/badge.svg)](https://docs.rs/rat_quickdb)
[![License](https://img.shields.io/crates/l/rat_quickdb.svg)](https://github.com/0ldm0s/rat_quickdb/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/rat_quickdb.svg)](https://crates.io/crates/rat_quickdb)

🚀 SQLite、PostgreSQL、MySQL、MongoDB対応の強力なクロスデータベースORMライブラリ

**🌐 言語バージョン**: [中文](README.md) | [English](README.en.md) | [日本語](README.ja.md)

## ✨ コア機能

- **🎯 自動インデックス作成**: モデル定義に基づいてテーブルとインデックスを自動作成、手動介入不要
- **🗄️ マルチデータベース対応**: SQLite、PostgreSQL、MySQL、MongoDB
- **🔗 統一API**: 異なるデータベースでも一貫したインターフェース
- **🏊 コネクションプール管理**: 効率的なコネクションプールとロックフリーキューアーキテクチャ
- **⚡ 非同期サポート**: Tokioベースの非同期ランタイム
- **🧠 スマートキャッシュ**: 組み込みキャッシュサポート（rat_memcacheベース）、TTL期限切れとフォールバック機構対応
- **🆔 複数のID生成戦略**: AutoIncrement、UUID、Snowflake、ObjectId、カスタム接頭辞
- **📝 ログ制御**: 呼び出し元による完全なログ初期化制御、ライブラリの自動初期化競合を回避
- **🐍 Pythonバインディング**: オプションのPython APIサポート
- **📋 タスクキュー**: 組み込み非同期タスクキューシステム
- **🔍 型安全性**: 強力な型モデル定義と検証

## 📦 インストール

`Cargo.toml`に依存関係を追加：

```toml
[dependencies]
rat_quickdb = "0.1.8"
```

## 🚀 クイックスタート

### 基本的な使用方法

```rust
use rat_quickdb::*;

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // ライブラリを初期化
    init();

    // SQLiteデータベース接続を追加
    let config = sqlite_config(
        "main",
        ":memory:",
        PoolConfig::default()
    )?;
    add_database(config).await?;

    // ユーザーデータを作成
    let mut user_data = HashMap::new();
    user_data.insert("name".to_string(), DataValue::String("田中太郎".to_string()));
    user_data.insert("email".to_string(), DataValue::String("tanaka@example.com".to_string()));

    // ユーザーレコードを作成
    create("users", user_data, Some("main")).await?;

    // ユーザーをクエリ
    let user = find_by_id("users", "1", Some("main")).await?;
    println!("見つかったユーザー: {:?}", user);

    Ok(())
}
```

### モデル定義（推奨）

```rust
use rat_quickdb::*;
use serde::{Serialize, Deserialize};

// ユーザーモデルを定義
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

    // データベースを追加
    let config = sqlite_config("main", "test.db", PoolConfig::default())?;
    add_database(config).await?;

    // ユーザーを作成（自動的にテーブルとインデックスを作成）
    let user = User {
        id: None,
        username: "tanaka".to_string(),
        email: "tanaka@example.com".to_string(),
        age: 25,
        is_active: true,
    };

    // ユーザーを保存（すべてのデータベース操作を自動処理）
    let user_id = user.save().await?;
    println!("ユーザーが正常に作成されました、ID: {}", user_id);

    // ユーザーをクエリ
    if let Some(found_user) = ModelManager::<User>::find_by_id(&user_id).await? {
        println!("見つかったユーザー: {} ({})", found_user.username, found_user.email);
    }

    Ok(())
}
```

## 🔧 データベース設定

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

#### 基本設定（コンビニエンス関数を使用）
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

#### 高度な設定（ビルダーを使用）
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

## 🛠️ コアAPI

### データベース管理
- `init()` - ライブラリを初期化
- `add_database(config)` - データベース設定を追加
- `remove_database(alias)` - データベース設定を削除
- `get_aliases()` - すべてのデータベースエイリアスを取得
- `set_default_alias(alias)` - デフォルトデータベースエイリアスを設定

### モデル操作（推奨）
```rust
// レコードを保存
let user_id = user.save().await?;

// レコードをクエリ
let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
let users = ModelManager::<User>::find(conditions, options).await?;

// レコードを更新
let mut updates = HashMap::new();
updates.insert("username".to_string(), DataValue::String("新しい名前".to_string()));
let updated = user.update(updates).await?;

// レコードを削除
let deleted = user.delete().await?;
```

### ODM操作（低レベル）
- `create(collection, data, alias)` - レコードを作成
- `find_by_id(collection, id, alias)` - IDで検索
- `find(collection, conditions, options, alias)` - レコードをクエリ
- `update(collection, id, data, alias)` - レコードを更新
- `delete(collection, id, alias)` - レコードを削除
- `count(collection, query, alias)` - レコード数をカウント
- `exists(collection, query, alias)` - 存在チェック

## 🏗️ アーキテクチャ機能

rat_quickdbはモダンアーキテクチャ設計を採用：

1. **ロックフリーキューアーキテクチャ**: 直接のデータベース接続ライフサイクル問題を回避
2. **モデル自動登録**: 初回使用時にモデルメタデータを自動登録
3. **自動インデックス管理**: モデル定義に基づいてテーブルとインデックスを自動作成
4. **クロスデータベースアダプタ**: 複数のデータベースタイプをサポートする統一インターフェース
5. **非同期メッセージ処理**: Tokioベースの効率的な非同期処理

## 🔄 ワークフロー

```
アプリケーション層 → モデル操作 → ODM層 → メッセージキュー → コネクションプール → データベース
    ↑                                                       ↓
    └────────────────── 結果返却 ←────────────────────────────┘
```

## 📊 パフォーマンス機能

- **コネクションプール管理**: インテリジェントなコネクション再利用と管理
- **非同期操作**: ノンブロッキングデータベース操作
- **バッチ処理**: バッチ操作最適化をサポート
- **キャッシュ統合**: 組み込みキャッシュでデータベースアクセスを削減
- **圧縮サポート**: MongoDBはZSTD圧縮をサポート

## 🎯 サポートされるフィールドタイプ

- `integer_field` - 整数フィールド（範囲と制約付き）
- `string_field` - 文字列フィールド（長さ制限付き、長い長さを設定してテキストとして使用可能）
- `float_field` - 浮動小数点数フィールド（範囲と精度付き）
- `boolean_field` - ブールフィールド
- `datetime_field` - 日時フィールド
- `uuid_field` - UUIDフィールド
- `json_field` - JSONフィールド
- `array_field` - 配列フィールド
- `list_field` - リストフィールド（array_fieldのエイリアス）
- `dict_field` - 辞書/オブジェクトフィールド（object_fieldの代替）
- `reference_field` - 参照フィールド（外部キー）

## 📝 インデックスサポート

- **ユニークインデックス**: `unique()` 制約
- **複合インデックス**: マルチフィールド組み合わせインデックス
- **通常インデックス**: 基本クエリ最適化インデックス
- **自動作成**: モデル定義に基づいて自動作成
- **クロスデータベース**: すべてのデータベースインデックスタイプをサポート

## 🌟 バージョン情報

**現在のバージョン**: 0.1.8

**サポートRustバージョン**: 1.70+

**重要なアップデート**: v0.1.8はID生成戦略、キャッシュ設定、ログ制御を改善し、すべてのコア機能を検証完了！

## 📄 ライセンス

このプロジェクトは[LGPL-v3](LICENSE)ライセンスの下で提供されています。

## 🤝 コントリビューション

このプロジェクトを改善するためのIssueやPull Requestの提出を歓迎します！

## 📞 お問い合わせ

質問や提案については、以下の方法でお問い合わせください：
- Issue作成: [GitHub Issues](https://github.com/your-repo/rat_quickdb/issues)
- メール: oldmos@gmail.com