//! 错误处理模块
//!
//! 提供统一的错误类型定义和中文错误信息

use thiserror::Error;

/// QuickDB 统一错误类型
#[derive(Error, Debug)]
pub enum QuickDbError {
    /// 数据库连接错误
    #[error("数据库连接失败: {message}")]
    ConnectionError { message: String },

    /// 连接池错误
    #[error("连接池操作失败: {message}")]
    PoolError { message: String },

    /// 查询执行错误
    #[error("查询执行失败: {message}")]
    QueryError { message: String },

    /// 序列化/反序列化错误
    #[error("数据序列化失败: {message}")]
    SerializationError { message: String },

    /// 模型验证错误
    #[error("模型验证失败: {field} - {message}")]
    ValidationError { field: String, message: String },

    /// 配置错误
    #[error("配置错误: {message}")]
    ConfigError { message: String },

    /// 数据库别名未找到
    #[error("数据库别名 '{alias}' 未找到")]
    AliasNotFound { alias: String },

    /// 不支持的数据库类型
    #[error("不支持的数据库类型: {db_type}")]
    UnsupportedDatabase { db_type: String },

    /// 事务操作错误（虽然不支持事务，但保留用于未来扩展）
    #[error("事务操作失败: {message}")]
    TransactionError { message: String },

    /// 任务执行错误
    #[error("任务执行失败: {0}")]
    TaskExecutionError(String),

    /// IO 错误
    #[error("IO 操作失败: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON 序列化错误
    #[error("JSON 处理失败: {0}")]
    JsonError(#[from] serde_json::Error),

    /// 通用错误
    #[error("操作失败: {0}")]
    Other(#[from] anyhow::Error),
}

/// QuickDB 结果类型别名
pub type QuickDbResult<T> = Result<T, QuickDbError>;

/// 错误构建器 - 提供便捷的错误创建方法
pub struct ErrorBuilder;

impl ErrorBuilder {
    /// 创建连接错误
    pub fn connection_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::ConnectionError {
            message: message.into(),
        }
    }

    /// 创建连接池错误
    pub fn pool_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::PoolError {
            message: message.into(),
        }
    }

    /// 创建查询错误
    pub fn query_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::QueryError {
            message: message.into(),
        }
    }

    /// 创建序列化错误
    pub fn serialization_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::SerializationError {
            message: message.into(),
        }
    }

    /// 创建验证错误
    pub fn validation_error(
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> QuickDbError {
        QuickDbError::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }

    /// 创建配置错误
    pub fn config_error(message: impl Into<String>) -> QuickDbError {
        QuickDbError::ConfigError {
            message: message.into(),
        }
    }

    /// 创建别名未找到错误
    pub fn alias_not_found(alias: impl Into<String>) -> QuickDbError {
        QuickDbError::AliasNotFound {
            alias: alias.into(),
        }
    }

    /// 创建不支持的数据库类型错误
    pub fn unsupported_database(db_type: impl Into<String>) -> QuickDbError {
        QuickDbError::UnsupportedDatabase {
            db_type: db_type.into(),
        }
    }
}

/// 便捷宏 - 快速创建错误
#[macro_export]
macro_rules! quick_error {
    (connection, $msg:expr) => {
        $crate::error::ErrorBuilder::connection_error($msg)
    };
    (pool, $msg:expr) => {
        $crate::error::ErrorBuilder::pool_error($msg)
    };
    (query, $msg:expr) => {
        $crate::error::ErrorBuilder::query_error($msg)
    };
    (serialization, $msg:expr) => {
        $crate::error::ErrorBuilder::serialization_error($msg)
    };
    (validation, $field:expr, $msg:expr) => {
        $crate::error::ErrorBuilder::validation_error($field, $msg)
    };
    (config, $msg:expr) => {
        $crate::error::ErrorBuilder::config_error($msg)
    };
    (alias_not_found, $alias:expr) => {
        $crate::error::ErrorBuilder::alias_not_found($alias)
    };
    (unsupported_db, $db_type:expr) => {
        $crate::error::ErrorBuilder::unsupported_database($db_type)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ErrorBuilder::connection_error("测试连接失败");
        assert!(matches!(err, QuickDbError::ConnectionError { .. }));
        assert_eq!(err.to_string(), "数据库连接失败: 测试连接失败");
    }

    #[test]
    fn test_error_macro() {
        let err = quick_error!(validation, "用户名", "不能为空");
        assert!(matches!(err, QuickDbError::ValidationError { .. }));
        assert_eq!(err.to_string(), "模型验证失败: 用户名 - 不能为空");
    }
}