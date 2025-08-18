//! 无锁队列桥接层
//! 
//! 使用 crossbeam::SegQueue 实现 Rust-Python 解耦通信
//! 通过 JSON 字符串传递数据，简化类型转换

use pyo3::prelude::*;
use crossbeam_queue::SegQueue;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use serde_json;
use uuid::Uuid;
use crate::task_queue::{get_global_task_queue, TaskQueueManager};

/// Python 请求消息
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Python 队列桥接器
#[pyclass(name = "QueueBridge")]
pub struct PyQueueBridge {
    /// 请求队列 - Python 向 Rust 发送请求
    request_queue: Arc<SegQueue<PyRequestMessage>>,
    /// 响应队列 - Rust 向 Python 返回响应
    response_queue: Arc<SegQueue<PyResponseMessage>>,
    /// 工作线程句柄
    worker_handle: Option<std::thread::JoinHandle<()>>,
}

// 手动实现 Clone trait
impl Clone for PyQueueBridge {
    fn clone(&self) -> Self {
        Self {
            request_queue: self.request_queue.clone(),
            response_queue: self.response_queue.clone(),
            worker_handle: None, // 工作线程句柄不能克隆
        }
    }
}

#[pymethods]
impl PyQueueBridge {
    /// 创建新的队列桥接器
    #[new]
    pub fn new() -> PyResult<Self> {
        let request_queue = Arc::new(SegQueue::new());
        let response_queue = Arc::new(SegQueue::new());
        
        Ok(Self {
            request_queue,
            response_queue,
            worker_handle: None,
        })
    }
    
    /// 启动工作线程
    fn start_worker(&mut self) -> PyResult<()> {
        if self.worker_handle.is_some() {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "工作线程已经启动"
            ));
        }
        
        let request_queue = self.request_queue.clone();
        let response_queue = self.response_queue.clone();
        
        let handle = std::thread::spawn(move || {
            Self::worker_thread(request_queue, response_queue);
        });
        
        self.worker_handle = Some(handle);
        
        Ok(())
    }
    
    /// 停止工作线程
    fn stop_worker(&mut self) -> PyResult<()> {
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }
    
    /// 发送请求并等待响应
    pub fn send_request(&self, request_type: String, data: String) -> PyResult<String> {
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyRequestMessage {
            request_id: request_id.clone(),
            request_type,
            data,
        };
        
        // 发送请求
        self.request_queue.push(request);
        
        // 等待响应
        let start_time = Instant::now();
        let timeout = Duration::from_secs(30); // 30秒超时
        
        loop {
            if let Some(response) = self.response_queue.pop() {
                if response.request_id == request_id {
                    if response.success {
                        return Ok(response.data);
                    } else {
                        return Err(pyo3::exceptions::PyRuntimeError::new_err(
                            response.error.unwrap_or("Unknown error".to_string())
                        ));
                    }
                }
                // 如果不是我们的响应，重新放回队列
                self.response_queue.push(response);
            }
            
            if start_time.elapsed() > timeout {
                return Err(pyo3::exceptions::PyTimeoutError::new_err("Request timeout"));
            }
            
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    
    /// 获取队列统计信息
    fn get_queue_stats(&self) -> PyResult<(usize, usize)> {
        let request_len = self.request_queue.len();
        let response_len = self.response_queue.len();
        
        Ok((request_len, response_len))
    }
}

impl PyQueueBridge {
    /// 工作线程主循环
    fn worker_thread(
        request_queue: Arc<SegQueue<PyRequestMessage>>,
        response_queue: Arc<SegQueue<PyResponseMessage>>,
    ) {
        loop {
            // 处理请求队列
            while let Some(request) = request_queue.pop() {
                let response = Self::process_request(request);
                response_queue.push(response);
            }
            
            // 短暂休眠避免CPU占用过高
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    
    /// 处理单个请求
    fn process_request(request: PyRequestMessage) -> PyResponseMessage {
        // 使用tokio运行时处理异步操作
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                return PyResponseMessage {
                    request_id: request.request_id,
                    success: false,
                    data: String::new(),
                    error: Some(format!("创建运行时失败: {}", e)),
                };
            }
        };
        
        // 在异步上下文中处理请求
        let result = rt.block_on(async {
            Self::process_request_async(request.clone()).await
        });
        
        match result {
            Ok(data) => PyResponseMessage {
                request_id: request.request_id,
                success: true,
                data,
                error: None,
            },
            Err(e) => PyResponseMessage {
                request_id: request.request_id,
                success: false,
                data: String::new(),
                error: Some(e.to_string()),
            },
        }
    }
    
    /// 异步处理请求
    async fn process_request_async(request: PyRequestMessage) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use crate::manager::get_global_pool_manager;
        use crate::pool::DatabaseOperation;
        use crate::types::{DataValue, QueryCondition};
        use tokio::sync::oneshot;
        use std::collections::HashMap;
        
        // 解析请求数据
        let request_data: serde_json::Value = serde_json::from_str(&request.data)
            .map_err(|e| format!("解析请求数据失败: {}", e))?;
        
        match request.request_type.as_str() {
            "create" => {
                // 解析创建请求
                let table = request_data["table"].as_str()
                    .ok_or("缺少表名")?;
                let data_obj = request_data["data"].as_object()
                    .ok_or("缺少数据对象")?;
                
                // 转换数据格式
                let mut data = HashMap::new();
                for (key, value) in data_obj {
                    let data_value = match value {
                        serde_json::Value::String(s) => DataValue::String(s.clone()),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                DataValue::Integer(i)
                            } else if let Some(f) = n.as_f64() {
                                DataValue::Float(f)
                            } else {
                                DataValue::String(n.to_string())
                            }
                        },
                        serde_json::Value::Bool(b) => DataValue::Boolean(*b),
                        serde_json::Value::Null => DataValue::Null,
                        _ => DataValue::String(value.to_string()),
                    };
                    data.insert(key.clone(), data_value);
                }
                
                // 获取连接池
                let manager = get_global_pool_manager();
                let connection_pools = manager.get_connection_pools();
                let connection_pool = connection_pools.get("default")
                    .ok_or("默认连接池不存在")?;
                
                // 创建oneshot通道
                let (response_tx, response_rx) = oneshot::channel();
                
                // 发送操作到连接池
                let operation = DatabaseOperation::Create {
                    table: table.to_string(),
                    data,
                    response: response_tx,
                };
                
                connection_pool.operation_sender.send(operation)
                    .map_err(|_| "连接池操作通道已关闭")?;
                
                // 等待响应
                let result = response_rx.await
                    .map_err(|_| "等待连接池响应超时")?
                    .map_err(|e| format!("数据库操作失败: {}", e))?;
                
                Ok(serde_json::to_string(&result)
                    .map_err(|e| format!("序列化结果失败: {}", e))?)
            },
            "find" => {
                // 解析查询请求
                let table = request_data["table"].as_str()
                    .ok_or("缺少表名")?;
                let conditions_array = request_data["conditions"].as_array()
                    .unwrap_or(&vec![]);
                
                // 转换查询条件
                let mut conditions = Vec::new();
                for condition in conditions_array {
                    if let (Some(field), Some(operator), Some(value)) = (
                        condition["field"].as_str(),
                        condition["operator"].as_str(),
                        &condition["value"]
                    ) {
                        let data_value = match value {
                            serde_json::Value::String(s) => DataValue::String(s.clone()),
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    DataValue::Integer(i)
                                } else if let Some(f) = n.as_f64() {
                                    DataValue::Float(f)
                                } else {
                                    DataValue::String(n.to_string())
                                }
                            },
                            serde_json::Value::Bool(b) => DataValue::Boolean(*b),
                            serde_json::Value::Null => DataValue::Null,
                            _ => DataValue::String(value.to_string()),
                        };
                        
                        let query_operator = match operator {
                            "eq" => crate::types::QueryOperator::Equal,
                            "ne" => crate::types::QueryOperator::NotEqual,
                            "gt" => crate::types::QueryOperator::GreaterThan,
                            "gte" => crate::types::QueryOperator::GreaterThanOrEqual,
                            "lt" => crate::types::QueryOperator::LessThan,
                            "lte" => crate::types::QueryOperator::LessThanOrEqual,
                            "like" => crate::types::QueryOperator::Like,
                            "in" => crate::types::QueryOperator::In,
                            "not_in" => crate::types::QueryOperator::NotIn,
                            _ => crate::types::QueryOperator::Equal,
                        };
                        
                        conditions.push(QueryCondition {
                            field: field.to_string(),
                            operator: query_operator,
                            value: data_value,
                        });
                    }
                }
                
                // 获取连接池
                let manager = get_global_pool_manager();
                let connection_pools = manager.get_connection_pools();
                let connection_pool = connection_pools.get("default")
                    .ok_or("默认连接池不存在")?;
                
                // 创建oneshot通道
                let (response_tx, response_rx) = oneshot::channel();
                
                // 发送操作到连接池
                let operation = DatabaseOperation::Find {
                    table: table.to_string(),
                    conditions,
                    options: Default::default(),
                    response: response_tx,
                };
                
                connection_pool.operation_sender.send(operation)
                    .map_err(|_| "连接池操作通道已关闭")?;
                
                // 等待响应
                let result = response_rx.await
                    .map_err(|_| "等待连接池响应超时")?
                    .map_err(|e| format!("数据库操作失败: {}", e))?;
                
                Ok(serde_json::to_string(&result)
                    .map_err(|e| format!("序列化结果失败: {}", e))?)
            },
            "find_by_id" => {
                // 解析根据ID查询请求
                let table = request_data["table"].as_str()
                    .ok_or("缺少表名")?;
                let id = request_data["id"].as_str()
                    .ok_or("缺少ID")?;
                
                // 获取连接池
                let manager = get_global_pool_manager();
                let connection_pools = manager.get_connection_pools();
                let connection_pool = connection_pools.get("default")
                    .ok_or("默认连接池不存在")?;
                
                // 创建oneshot通道
                let (response_tx, response_rx) = oneshot::channel();
                
                // 发送操作到连接池
                let operation = DatabaseOperation::FindById {
                    table: table.to_string(),
                    id: DataValue::String(id.to_string()),
                    response: response_tx,
                };
                
                connection_pool.operation_sender.send(operation)
                    .map_err(|_| "连接池操作通道已关闭")?;
                
                // 等待响应
                let result = response_rx.await
                    .map_err(|_| "等待连接池响应超时")?
                    .map_err(|e| format!("数据库操作失败: {}", e))?;
                
                Ok(serde_json::to_string(&result)
                    .map_err(|e| format!("序列化结果失败: {}", e))?)
            },
            _ => {
                Err(format!("不支持的请求类型: {}", request.request_type).into())
            }
        }
    }
}

/// 创建队列桥接器的工厂函数
#[pyfunction]
pub fn create_queue_bridge() -> PyResult<PyQueueBridge> {
    Ok(PyQueueBridge::new()?)
}