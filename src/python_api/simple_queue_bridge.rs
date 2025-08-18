//! 简化版队列桥接器
//! 
//! 使用 crossbeam::SegQueue 实现 Rust-Python 解耦通信
//! 移除复杂的任务队列依赖，直接处理基本数据库操作

use pyo3::prelude::*;
use crossbeam_queue::SegQueue;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde_json;
use uuid::Uuid;
use std::collections::HashMap;
use zerg_creep::{info, warn, error};

use crate::types::{DataValue, QueryCondition, QueryOperator};

/// Python 请求消息
#[derive(Debug, Clone)]
pub struct PyRequestMessage {
    /// 请求ID
    pub request_id: String,
    /// 请求类型
    pub request_type: String,
    /// 请求数据（JSON字符串）
    pub data: String,
}

/// Python 响应消息
#[derive(Debug, Clone)]
pub struct PyResponseMessage {
    /// 请求ID
    pub request_id: String,
    /// 是否成功
    pub success: bool,
    /// 响应数据（JSON字符串）
    pub data: String,
    /// 错误信息
    pub error: Option<String>,
}

/// 简化版 Python 队列桥接器
#[pyclass(name = "SimpleQueueBridge")]
pub struct PySimpleQueueBridge {
    /// 请求队列 - Python 向 Rust 发送请求
    request_queue: Arc<SegQueue<PyRequestMessage>>,
    /// 响应队列 - Rust 向 Python 返回响应
    response_queue: Arc<SegQueue<PyResponseMessage>>,
    /// 模拟数据存储
    data_store: Arc<std::sync::Mutex<HashMap<String, HashMap<String, serde_json::Value>>>>,
}

#[pymethods]
impl PySimpleQueueBridge {
    /// 创建新的简化队列桥接器
    #[new]
    pub fn new() -> PyResult<Self> {
        info!("创建简化版队列桥接器");
        
        let request_queue = Arc::new(SegQueue::new());
        let response_queue = Arc::new(SegQueue::new());
        let data_store = Arc::new(std::sync::Mutex::new(HashMap::new()));
        
        Ok(Self {
            request_queue,
            response_queue,
            data_store,
        })
    }
    
    /// 发送请求并等待响应
    pub fn send_request(&self, request_type: String, data: String) -> PyResult<String> {
        let request_id = Uuid::new_v4().to_string();
        
        info!("发送请求: {} - {}", request_type, request_id);
        
        let request = PyRequestMessage {
            request_id: request_id.clone(),
            request_type: request_type.clone(),
            data: data.clone(),
        };
        
        // 直接处理请求，不使用工作线程
        let response = self.process_request_sync(request);
        
        if response.success {
            Ok(response.data)
        } else {
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                response.error.unwrap_or("未知错误".to_string())
            ))
        }
    }
    
    /// 创建记录
    pub fn create_record(&self, table: String, data: String) -> PyResult<String> {
        self.send_request("create".to_string(), serde_json::json!({
            "table": table,
            "data": data
        }).to_string())
    }
    
    /// 查询记录
    pub fn find_records(&self, table: String, conditions: String) -> PyResult<String> {
        self.send_request("find".to_string(), serde_json::json!({
            "table": table,
            "conditions": conditions
        }).to_string())
    }
    
    /// 更新记录
    pub fn update_records(&self, table: String, conditions: String, data: String) -> PyResult<String> {
        self.send_request("update".to_string(), serde_json::json!({
            "table": table,
            "conditions": conditions,
            "data": data
        }).to_string())
    }
    
    /// 删除记录
    pub fn delete_records(&self, table: String, conditions: String) -> PyResult<String> {
        self.send_request("delete".to_string(), serde_json::json!({
            "table": table,
            "conditions": conditions
        }).to_string())
    }
    
    /// 统计记录数量
    pub fn count_records(&self, table: String, conditions: String) -> PyResult<u64> {
        let result = self.send_request("count".to_string(), serde_json::json!({
            "table": table,
            "conditions": conditions
        }).to_string())?;
        
        result.parse::<u64>().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("解析计数结果失败: {}", e))
        })
    }
    
    /// 检查记录是否存在
    pub fn record_exists(&self, table: String, conditions: String) -> PyResult<bool> {
        let result = self.send_request("exists".to_string(), serde_json::json!({
            "table": table,
            "conditions": conditions
        }).to_string())?;
        
        result.parse::<bool>().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("解析存在性结果失败: {}", e))
        })
    }
    
    /// 获取队列统计信息
    pub fn get_queue_stats(&self) -> PyResult<(usize, usize)> {
        let request_len = self.request_queue.len();
        let response_len = self.response_queue.len();
        
        Ok((request_len, response_len))
    }
    
    /// 测试连接
    pub fn test_connection(&self) -> PyResult<String> {
        info!("测试简化队列桥接器连接");
        Ok("简化队列桥接器连接正常".to_string())
    }
}

impl PySimpleQueueBridge {
    /// 同步处理请求
    fn process_request_sync(&self, request: PyRequestMessage) -> PyResponseMessage {
        info!("处理请求: {} - {}", request.request_type, request.request_id);
        
        let result = match request.request_type.as_str() {
            "create" => self.handle_create(&request.data),
            "find" => self.handle_find(&request.data),
            "update" => self.handle_update(&request.data),
            "delete" => self.handle_delete(&request.data),
            "count" => self.handle_count(&request.data),
            "exists" => self.handle_exists(&request.data),
            _ => Err(format!("不支持的请求类型: {}", request.request_type)),
        };
        
        match result {
            Ok(data) => PyResponseMessage {
                request_id: request.request_id,
                success: true,
                data,
                error: None,
            },
            Err(error) => {
                error!("处理请求失败: {}", error);
                PyResponseMessage {
                    request_id: request.request_id,
                    success: false,
                    data: String::new(),
                    error: Some(error),
                }
            }
        }
    }
    
    /// 处理创建操作
    fn handle_create(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析创建请求失败: {}", e))?;
        
        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        let record_data = request["data"].as_str()
            .ok_or("缺少记录数据")?;
        
        let record: serde_json::Value = serde_json::from_str(record_data)
            .map_err(|e| format!("解析记录数据失败: {}", e))?;
        
        let mut store = self.data_store.lock().unwrap();
        let table_data = store.entry(table.to_string()).or_insert_with(HashMap::new);
        
        let id = Uuid::new_v4().to_string();
        let mut record_with_id = record.as_object().unwrap().clone();
        record_with_id.insert("id".to_string(), serde_json::Value::String(id.clone()));
        record_with_id.insert("created_at".to_string(), 
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
        
        table_data.insert(id.clone(), serde_json::Value::Object(record_with_id));
        
        info!("创建记录成功: {} - {}", table, id);
        Ok(id)
    }
    
    /// 处理查询操作
    fn handle_find(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析查询请求失败: {}", e))?;
        
        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        
        let store = self.data_store.lock().unwrap();
        let empty_table = HashMap::new();
        let table_data = store.get(table).unwrap_or(&empty_table);
        
        let records: Vec<&serde_json::Value> = table_data.values().collect();
        let result = serde_json::to_string(&records)
            .map_err(|e| format!("序列化查询结果失败: {}", e))?;
        
        info!("查询记录成功: {} - {} 条记录", table, records.len());
        Ok(result)
    }
    
    /// 处理更新操作
    fn handle_update(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析更新请求失败: {}", e))?;
        
        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        
        let mut store = self.data_store.lock().unwrap();
        let table_data = store.entry(table.to_string()).or_insert_with(HashMap::new);
        
        let updated_count = table_data.len();
        
        // 简化实现：更新所有记录的 updated_at 字段
        for (_, record) in table_data.iter_mut() {
            if let Some(obj) = record.as_object_mut() {
                obj.insert("updated_at".to_string(), 
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            }
        }
        
        info!("更新记录成功: {} - {} 条记录", table, updated_count);
        Ok(updated_count.to_string())
    }
    
    /// 处理删除操作
    fn handle_delete(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析删除请求失败: {}", e))?;
        
        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        
        let mut store = self.data_store.lock().unwrap();
        let deleted_count = if let Some(table_data) = store.get_mut(table) {
            let count = table_data.len();
            table_data.clear();
            count
        } else {
            0
        };
        
        info!("删除记录成功: {} - {} 条记录", table, deleted_count);
        Ok(deleted_count.to_string())
    }
    
    /// 处理计数操作
    fn handle_count(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析计数请求失败: {}", e))?;
        
        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        
        let store = self.data_store.lock().unwrap();
        let count = store.get(table).map(|t| t.len()).unwrap_or(0);
        
        info!("计数记录成功: {} - {} 条记录", table, count);
        Ok(count.to_string())
    }
    
    /// 处理存在性检查操作
    fn handle_exists(&self, data: &str) -> Result<String, String> {
        let request: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| format!("解析存在性检查请求失败: {}", e))?;
        
        let table = request["table"].as_str()
            .ok_or("缺少表名")?;
        
        let store = self.data_store.lock().unwrap();
        let exists = store.get(table).map(|t| !t.is_empty()).unwrap_or(false);
        
        info!("存在性检查成功: {} - {}", table, exists);
        Ok(exists.to_string())
    }
}

/// 创建简化队列桥接器的工厂函数
#[pyfunction]
pub fn create_simple_queue_bridge() -> PyResult<PySimpleQueueBridge> {
    info!("创建简化队列桥接器实例");
    Ok(PySimpleQueueBridge::new()?)
}