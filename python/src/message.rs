use rat_quickdb::types::{DataValue, QueryCondition, QueryConditionGroup, QueryOptions, DatabaseConfig};
use rat_quickdb::model::FieldType;
use std::collections::HashMap;
use tokio::sync::oneshot;

/// Python请求消息结构
#[derive(Debug, Clone)]
pub struct PyRequestMessage {
    pub id: String,
    pub request_type: String,
    pub data: String,
}

/// Python响应消息结构
#[derive(Debug, Clone)]
pub struct PyResponseMessage {
    pub id: String,
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
}

/// 数据库请求结构
#[derive(Debug)]
pub struct PyDbRequest {
    /// 请求ID
    pub request_id: String,
    /// 操作类型
    pub operation: String,
    /// 集合名称
    pub collection: String,
    /// 数据
    pub data: Option<HashMap<String, DataValue>>,
    /// 查询条件
    pub conditions: Option<Vec<QueryCondition>>,
    /// 查询条件组合（支持OR逻辑）
    pub condition_groups: Option<Vec<QueryConditionGroup>>,
    /// 查询选项
    pub options: Option<QueryOptions>,
    /// 更新数据
    pub updates: Option<HashMap<String, DataValue>>,
    /// ID
    pub id: Option<String>,
    /// 数据库别名
    pub alias: Option<String>,
    /// 数据库配置（用于add_database操作）
    pub database_config: Option<DatabaseConfig>,
    /// 表字段定义（用于create_table操作）
    pub fields: Option<HashMap<String, rat_quickdb::model::FieldType>>,
    /// 响应发送器
    pub response_sender: oneshot::Sender<PyDbResponse>,
}

/// 数据库响应结构
#[derive(Debug, Clone)]
pub struct PyDbResponse {
    /// 请求ID
    pub request_id: String,
    /// 是否成功
    pub success: bool,
    /// 响应数据
    pub data: String,
    /// 错误信息
    pub error: Option<String>,
}