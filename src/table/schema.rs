//! 表模式定义
//! 
//! 定义表结构、列类型、索引和约束

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::DataValue;
use chrono::{DateTime, Utc};

/// 表模式定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableSchema {
    /// 表名
    pub name: String,
    /// 列定义
    pub columns: Vec<ColumnDefinition>,
    /// 索引定义
    pub indexes: Vec<IndexDefinition>,
    /// 约束定义
    pub constraints: Vec<ConstraintDefinition>,
    /// 表选项
    pub options: TableOptions,
    /// 模式版本
    pub version: u32,
    /// 创建时间
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 最后修改时间
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 列定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnDefinition {
    /// 列名
    pub name: String,
    /// 列类型
    pub column_type: ColumnType,
    /// 是否可为空
    pub nullable: bool,
    /// 默认值
    pub default_value: Option<String>,
    /// 是否为主键
    pub primary_key: bool,
    /// 是否自增
    pub auto_increment: bool,
    /// 是否唯一
    pub unique: bool,
    /// 列注释
    pub comment: Option<String>,
    /// 列长度（用于字符串类型）
    pub length: Option<u32>,
    /// 精度（用于数值类型）
    pub precision: Option<u32>,
    /// 小数位数（用于数值类型）
    pub scale: Option<u32>,
}

/// 列类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColumnType {
    /// 整数类型
    Integer,
    /// 大整数类型
    BigInteger,
    /// 小整数类型
    SmallInteger,
    /// 浮点数类型
    Float,
    /// 双精度浮点数类型
    Double,
    /// 精确数值类型
    Decimal { precision: u32, scale: u32 },
    /// 字符串类型
    String { length: Option<u32> },
    /// 文本类型
    Text,
    /// 长文本类型
    LongText,
    /// 布尔类型
    Boolean,
    /// 日期类型
    Date,
    /// 时间类型
    Time,
    /// 日期时间类型
    DateTime,
    /// 时间戳类型
    Timestamp,
    /// 二进制数据类型
    Binary { length: Option<u32> },
    /// 大二进制数据类型
    Blob,
    /// JSON类型
    Json,
    /// UUID类型
    Uuid,
    /// 枚举类型
    Enum { values: Vec<String> },
    /// 自定义类型
    Custom { type_name: String },
}

/// 索引定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexDefinition {
    /// 索引名
    pub name: String,
    /// 索引类型
    pub index_type: IndexType,
    /// 索引列
    pub columns: Vec<String>,
    /// 是否唯一
    pub unique: bool,
    /// 索引选项
    pub options: HashMap<String, String>,
}

/// 索引类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexType {
    /// B-Tree索引（默认）
    BTree,
    /// 哈希索引
    Hash,
    /// 全文索引
    FullText,
    /// 空间索引
    Spatial,
    /// 自定义索引类型
    Custom(String),
}

/// 约束定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstraintDefinition {
    /// 约束名
    pub name: String,
    /// 约束类型
    pub constraint_type: ConstraintType,
    /// 约束列
    pub columns: Vec<String>,
    /// 引用表（用于外键约束）
    pub reference_table: Option<String>,
    /// 引用列（用于外键约束）
    pub reference_columns: Option<Vec<String>>,
    /// 删除时的动作
    pub on_delete: Option<ReferentialAction>,
    /// 更新时的动作
    pub on_update: Option<ReferentialAction>,
    /// 检查条件（用于检查约束）
    pub check_condition: Option<String>,
}

/// 约束类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstraintType {
    /// 主键约束
    PrimaryKey,
    /// 外键约束
    ForeignKey,
    /// 唯一约束
    Unique,
    /// 检查约束
    Check,
    /// 非空约束
    NotNull,
}

/// 引用动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReferentialAction {
    /// 级联
    Cascade,
    /// 设置为NULL
    SetNull,
    /// 设置为默认值
    SetDefault,
    /// 限制（默认）
    Restrict,
    /// 无动作
    NoAction,
}

/// 表选项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableOptions {
    /// 存储引擎（MySQL）
    pub engine: Option<String>,
    /// 字符集
    pub charset: Option<String>,
    /// 排序规则
    pub collation: Option<String>,
    /// 表注释
    pub comment: Option<String>,
    /// 自增起始值
    pub auto_increment: Option<u64>,
    /// 行格式
    pub row_format: Option<String>,
    /// 其他选项
    pub extra_options: HashMap<String, String>,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            engine: None,
            charset: Some("utf8mb4".to_string()),
            collation: Some("utf8mb4_unicode_ci".to_string()),
            comment: None,
            auto_increment: None,
            row_format: None,
            extra_options: HashMap::new(),
        }
    }
}

impl TableSchema {
    /// 创建新的表模式
    pub fn new(name: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
            indexes: Vec::new(),
            constraints: Vec::new(),
            options: TableOptions::default(),
            version: 1,
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }
    
    /// 添加列
    pub fn add_column(mut self, column: ColumnDefinition) -> Self {
        self.columns.push(column);
        self.updated_at = Some(chrono::Utc::now());
        self
    }
    
    /// 添加索引
    pub fn add_index(mut self, index: IndexDefinition) -> Self {
        self.indexes.push(index);
        self.updated_at = Some(chrono::Utc::now());
        self
    }
    
    /// 添加约束
    pub fn add_constraint(mut self, constraint: ConstraintDefinition) -> Self {
        self.constraints.push(constraint);
        self.updated_at = Some(chrono::Utc::now());
        self
    }
    
    /// 设置表选项
    pub fn with_options(mut self, options: TableOptions) -> Self {
        self.options = options;
        self.updated_at = Some(chrono::Utc::now());
        self
    }
    
    /// 获取主键列
    pub fn get_primary_key_columns(&self) -> Vec<&ColumnDefinition> {
        self.columns.iter().filter(|col| col.primary_key).collect()
    }
    
    /// 检查列是否存在
    pub fn has_column(&self, column_name: &str) -> bool {
        self.columns.iter().any(|col| col.name == column_name)
    }
    
    /// 获取列定义
    pub fn get_column(&self, column_name: &str) -> Option<&ColumnDefinition> {
        self.columns.iter().find(|col| col.name == column_name)
    }
    
    /// 检查索引是否存在
    pub fn has_index(&self, index_name: &str) -> bool {
        self.indexes.iter().any(|idx| idx.name == index_name)
    }
    
    /// 获取索引定义
    pub fn get_index(&self, index_name: &str) -> Option<&IndexDefinition> {
        self.indexes.iter().find(|idx| idx.name == index_name)
    }
    
    /// 验证模式定义
    pub fn validate(&self) -> Result<(), String> {
        // 检查是否有列定义
        if self.columns.is_empty() {
            return Err("表必须至少有一个列".to_string());
        }
        
        // 检查列名是否重复
        let mut column_names = std::collections::HashSet::new();
        for column in &self.columns {
            if !column_names.insert(&column.name) {
                return Err(format!("列名 '{}' 重复", column.name));
            }
        }
        
        // 检查索引名是否重复
        let mut index_names = std::collections::HashSet::new();
        for index in &self.indexes {
            if !index_names.insert(&index.name) {
                return Err(format!("索引名 '{}' 重复", index.name));
            }
            
            // 检查索引列是否存在
            for column_name in &index.columns {
                if !self.has_column(column_name) {
                    return Err(format!("索引 '{}' 引用的列 '{}' 不存在", index.name, column_name));
                }
            }
        }
        
        // 检查约束名是否重复
        let mut constraint_names = std::collections::HashSet::new();
        for constraint in &self.constraints {
            if !constraint_names.insert(&constraint.name) {
                return Err(format!("约束名 '{}' 重复", constraint.name));
            }
            
            // 检查约束列是否存在
            for column_name in &constraint.columns {
                if !self.has_column(column_name) {
                    return Err(format!("约束 '{}' 引用的列 '{}' 不存在", constraint.name, column_name));
                }
            }
        }
        
        Ok(())
    }
    
    /// 从数据字段推断表结构
    pub fn infer_from_data(table_name: String, data: &HashMap<String, DataValue>) -> Self {
        let mut columns = Vec::new();
        let mut has_id_field = false;
        
        // 根据数据字段推断列类型
        for (field_name, field_value) in data {
            if field_name == "id" {
                has_id_field = true;
                // 如果数据中包含id字段，使用数据中的类型，但设置为主键
                let column_type = Self::infer_column_type(field_value);
                columns.push(ColumnDefinition {
                    name: field_name.clone(),
                    column_type,
                    nullable: false, // 主键不能为空
                    default_value: None,
                    primary_key: true,
                    auto_increment: matches!(field_value, DataValue::Int(_)), // 只有整数类型才自增
                    unique: true,
                    comment: Some("主键ID".to_string()),
                    length: None,
                    precision: None,
                    scale: None,
                });
            } else {
                let column_type = Self::infer_column_type(field_value);
                columns.push(ColumnDefinition {
                    name: field_name.clone(),
                    column_type,
                    nullable: matches!(field_value, DataValue::Null),
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    comment: None,
                    length: None,
                    precision: None,
                    scale: None,
                });
            }
        }
        
        // 如果数据中没有id字段，添加默认的id主键列
        if !has_id_field {
            columns.insert(0, ColumnDefinition {
                name: "id".to_string(),
                column_type: ColumnType::BigInteger,
                nullable: false,
                default_value: None,
                primary_key: true,
                auto_increment: true,
                unique: true,
                comment: Some("主键ID".to_string()),
                length: None,
                precision: None,
                scale: None,
            });
        }
        
        Self {
            name: table_name,
            columns,
            indexes: Vec::new(),
            constraints: Vec::new(),
            options: TableOptions::default(),
            version: 1,
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }
    
    /// 从DataValue推断列类型
    fn infer_column_type(value: &DataValue) -> ColumnType {
        match value {
            DataValue::Null => ColumnType::String { length: Some(255) }, // 默认为字符串
            DataValue::Bool(_) => ColumnType::Boolean,
            DataValue::Int(_) => ColumnType::BigInteger,
            DataValue::Float(_) => ColumnType::Double,
            DataValue::String(s) => {
                // 根据字符串长度决定类型
                if s.len() > 65535 {
                    ColumnType::LongText
                } else if s.len() > 255 {
                    ColumnType::Text
                } else {
                    ColumnType::String { length: Some(255) }
                }
            },
            DataValue::Bytes(_) => ColumnType::Blob,
            DataValue::DateTime(_) => ColumnType::DateTime,
            DataValue::Uuid(_) => ColumnType::Uuid,
            DataValue::Json(_) => ColumnType::Json,
            DataValue::Array(_) => ColumnType::Json, // 数组存储为JSON
            DataValue::Object(_) => ColumnType::Json, // 对象存储为JSON
        }
    }
}

impl ColumnDefinition {
    /// 创建新的列定义
    pub fn new(name: String, column_type: ColumnType) -> Self {
        Self {
            name,
            column_type,
            nullable: true,
            default_value: None,
            primary_key: false,
            auto_increment: false,
            unique: false,
            comment: None,
            length: None,
            precision: None,
            scale: None,
        }
    }
    
    /// 设置为主键
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }
    
    /// 设置为非空
    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }
    
    /// 设置为唯一
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
    
    /// 设置为自增
    pub fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self.nullable = false;
        self
    }
    
    /// 设置默认值
    pub fn default_value<T: ToString>(mut self, value: T) -> Self {
        self.default_value = Some(value.to_string());
        self
    }
    
    /// 设置注释
    pub fn comment<T: ToString>(mut self, comment: T) -> Self {
        self.comment = Some(comment.to_string());
        self
    }
}

impl IndexDefinition {
    /// 创建新的索引定义
    pub fn new(name: String, columns: Vec<String>) -> Self {
        Self {
            name,
            index_type: IndexType::BTree,
            columns,
            unique: false,
            options: HashMap::new(),
        }
    }
    
    /// 设置为唯一索引
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
    
    /// 设置索引类型
    pub fn index_type(mut self, index_type: IndexType) -> Self {
        self.index_type = index_type;
        self
    }
    
    /// 添加选项
    pub fn option<K: ToString, V: ToString>(mut self, key: K, value: V) -> Self {
        self.options.insert(key.to_string(), value.to_string());
        self
    }
}

impl ConstraintDefinition {
    /// 创建主键约束
    pub fn primary_key(name: String, columns: Vec<String>) -> Self {
        Self {
            name,
            constraint_type: ConstraintType::PrimaryKey,
            columns,
            reference_table: None,
            reference_columns: None,
            on_delete: None,
            on_update: None,
            check_condition: None,
        }
    }
    
    /// 创建外键约束
    pub fn foreign_key(
        name: String,
        columns: Vec<String>,
        reference_table: String,
        reference_columns: Vec<String>,
    ) -> Self {
        Self {
            name,
            constraint_type: ConstraintType::ForeignKey,
            columns,
            reference_table: Some(reference_table),
            reference_columns: Some(reference_columns),
            on_delete: Some(ReferentialAction::Restrict),
            on_update: Some(ReferentialAction::Restrict),
            check_condition: None,
        }
    }
    
    /// 创建唯一约束
    pub fn unique(name: String, columns: Vec<String>) -> Self {
        Self {
            name,
            constraint_type: ConstraintType::Unique,
            columns,
            reference_table: None,
            reference_columns: None,
            on_delete: None,
            on_update: None,
            check_condition: None,
        }
    }
    
    /// 创建检查约束
    pub fn check(name: String, condition: String) -> Self {
        Self {
            name,
            constraint_type: ConstraintType::Check,
            columns: Vec::new(),
            reference_table: None,
            reference_columns: None,
            on_delete: None,
            on_update: None,
            check_condition: Some(condition),
        }
    }
    
    /// 设置删除时的动作
    pub fn on_delete(mut self, action: ReferentialAction) -> Self {
        self.on_delete = Some(action);
        self
    }
    
    /// 设置更新时的动作
    pub fn on_update(mut self, action: ReferentialAction) -> Self {
        self.on_update = Some(action);
        self
    }
}