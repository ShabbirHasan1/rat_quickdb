//! 数据库操作任务定义
//! 
//! 定义所有可以通过任务队列执行的数据库操作

use std::collections::HashMap;
use tokio::sync::oneshot;
use serde::{Serialize, Deserialize};
use crate::error::QuickDbResult;
use crate::types::*;
use crate::table::TableSchema;

/// 数据库操作任务
#[derive(Debug)]
pub enum DbTask {
    /// 创建记录任务
    Create {
        /// 表名
        table: String,
        /// 数据
        data: HashMap<String, DataValue>,
        /// 创建选项
        options: Option<CreateOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<String>>,
    },
    
    /// 查询记录任务
    Find {
        /// 表名
        table: String,
        /// 查询条件
        conditions: Vec<QueryCondition>,
        /// 查询选项
        options: Option<QueryOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<String>>,
    },
    
    /// 根据ID查询记录任务
    FindById {
        /// 表名
        table: String,
        /// 记录ID
        id: String,
        /// 查询选项
        options: Option<QueryOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<Option<String>>>,
    },
    
    /// 更新记录任务
    Update {
        /// 表名
        table: String,
        /// 查询条件
        conditions: Vec<QueryCondition>,
        /// 更新数据
        data: HashMap<String, DataValue>,
        /// 更新选项
        options: Option<UpdateOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<u64>>,
    },
    
    /// 根据ID更新记录任务
    UpdateById {
        /// 表名
        table: String,
        /// 记录ID
        id: String,
        /// 更新数据
        data: HashMap<String, DataValue>,
        /// 更新选项
        options: Option<UpdateOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<bool>>,
    },
    
    /// 删除记录任务
    Delete {
        /// 表名
        table: String,
        /// 查询条件
        conditions: Vec<QueryCondition>,
        /// 删除选项
        options: Option<DeleteOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<u64>>,
    },
    
    /// 根据ID删除记录任务
    DeleteById {
        /// 表名
        table: String,
        /// 记录ID
        id: String,
        /// 删除选项
        options: Option<DeleteOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<bool>>,
    },
    
    /// 计数任务
    Count {
        /// 表名
        table: String,
        /// 查询条件
        conditions: Vec<QueryCondition>,
        /// 计数选项
        options: Option<CountOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<u64>>,
    },
    
    /// 存在性检查任务
    Exists {
        /// 表名
        table: String,
        /// 查询条件
        conditions: Vec<QueryCondition>,
        /// 存在性检查选项
        options: Option<ExistsOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<bool>>,
    },
    
    /// 批量创建任务
    BatchCreate {
        /// 表名
        table: String,
        /// 数据列表
        data_list: Vec<HashMap<String, DataValue>>,
        /// 创建选项
        options: Option<CreateOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<Vec<String>>>,
    },
    
    /// 批量更新任务
    BatchUpdate {
        /// 表名
        table: String,
        /// 更新项列表（条件和数据的配对）
        updates: Vec<(Vec<QueryCondition>, HashMap<String, DataValue>)>,
        /// 更新选项
        options: Option<UpdateOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<u64>>,
    },
    
    /// 批量删除任务
    BatchDelete {
        /// 表名
        table: String,
        /// 条件列表
        conditions_list: Vec<Vec<QueryCondition>>,
        /// 删除选项
        options: Option<DeleteOptions>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<u64>>,
    },
    
    // === 表管理任务 ===
    
    /// 检查表是否存在任务
    CheckTable {
        /// 表名
        table: String,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<bool>>,
    },
    
    /// 创建表任务
    CreateTable {
        /// 表名
        table: String,
        /// 表模式
        schema: TableSchema,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<()>>,
    },
    
    /// 删除表任务
    DropTable {
        /// 表名
        table: String,
        /// 是否级联删除
        cascade: bool,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<()>>,
    },
    
    /// 表迁移任务
    MigrateTable {
        /// 表名
        table: String,
        /// 源版本
        from_version: u32,
        /// 目标版本
        to_version: u32,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<()>>,
    },
    
    /// 获取表模式任务
    GetTableSchema {
        /// 表名
        table: String,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<Option<TableSchema>>>,
    },
    
    /// 列出所有表任务
    ListTables {
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<Vec<String>>>,
    },
    
    // === 事务任务 ===
    
    /// 开始事务任务
    BeginTransaction {
        /// 事务选项
        options: Option<TransactionOptions>,
        /// 结果返回通道（返回事务ID）
        response_tx: oneshot::Sender<QuickDbResult<String>>,
    },
    
    /// 提交事务任务
    CommitTransaction {
        /// 事务ID
        transaction_id: String,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<()>>,
    },
    
    /// 回滚事务任务
    RollbackTransaction {
        /// 事务ID
        transaction_id: String,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<()>>,
    },
    
    // === 系统任务 ===
    

    
    /// 执行原生SQL任务
    ExecuteRaw {
        /// SQL语句
        sql: String,
        /// 参数
        params: Vec<DataValue>,
        /// 结果返回通道
        response_tx: oneshot::Sender<QuickDbResult<String>>,
    },
    
    /// 关闭任务（用于优雅关闭工作线程）
    Shutdown,
}

/// 创建选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOptions {
    /// 是否返回创建的记录
    pub return_record: bool,
    /// 冲突时的处理策略
    pub on_conflict: ConflictStrategy,
    /// 超时时间（毫秒）
    pub timeout: Option<u64>,
}

/// 更新选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOptions {
    /// 是否返回更新的记录
    pub return_record: bool,
    /// 是否进行乐观锁检查
    pub optimistic_lock: bool,
    /// 超时时间（毫秒）
    pub timeout: Option<u64>,
}

/// 删除选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOptions {
    /// 是否软删除
    pub soft_delete: bool,
    /// 是否级联删除
    pub cascade: bool,
    /// 超时时间（毫秒）
    pub timeout: Option<u64>,
}

/// 计数选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountOptions {
    /// 是否使用近似计数（性能更好）
    pub approximate: bool,
    /// 超时时间（毫秒）
    pub timeout: Option<u64>,
}

/// 存在性检查选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistsOptions {
    /// 超时时间（毫秒）
    pub timeout: Option<u64>,
}

/// 事务选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOptions {
    /// 隔离级别
    pub isolation_level: IsolationLevel,
    /// 是否只读
    pub read_only: bool,
    /// 超时时间（毫秒）
    pub timeout: Option<u64>,
}

/// 冲突策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictStrategy {
    /// 抛出错误
    Error,
    /// 忽略冲突
    Ignore,
    /// 替换现有记录
    Replace,
    /// 更新现有记录
    Update,
}

/// 隔离级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// 读未提交
    ReadUncommitted,
    /// 读已提交
    ReadCommitted,
    /// 可重复读
    RepeatableRead,
    /// 串行化
    Serializable,
}



impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            return_record: false,
            on_conflict: ConflictStrategy::Error,
            timeout: Some(30000), // 30秒默认超时
        }
    }
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self {
            return_record: false,
            optimistic_lock: false,
            timeout: Some(30000),
        }
    }
}

impl Default for DeleteOptions {
    fn default() -> Self {
        Self {
            soft_delete: false,
            cascade: false,
            timeout: Some(30000),
        }
    }
}

impl Default for CountOptions {
    fn default() -> Self {
        Self {
            approximate: false,
            timeout: Some(30000),
        }
    }
}

impl Default for ExistsOptions {
    fn default() -> Self {
        Self {
            timeout: Some(30000),
        }
    }
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::ReadCommitted,
            read_only: false,
            timeout: Some(60000), // 事务默认60秒超时
        }
    }
}