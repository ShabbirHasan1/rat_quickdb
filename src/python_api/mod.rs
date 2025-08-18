//! Python API 模块
//! 
//! 提供简化的 Python 绑定，使用 JSON 字符串进行数据传递
//! 通过无锁队列与 Rust 底层解耦

use pyo3::prelude::*;

// 恢复简化版的队列桥接器
#[cfg(feature = "python-bindings")]
pub mod simple_queue_bridge;

#[cfg(feature = "python-bindings")]
pub use simple_queue_bridge::{PySimpleQueueBridge, create_simple_queue_bridge};

/// 简化的 Python 类，用于测试基本的 PyO3 功能
#[cfg(feature = "python-bindings")]
#[pyclass(name = "SimpleTest")]
pub struct PySimpleTest {
    value: String,
}

#[cfg(feature = "python-bindings")]
#[pymethods]
impl PySimpleTest {
    #[new]
    pub fn new(value: String) -> Self {
        Self { value }
    }
    
    pub fn get_value(&self) -> String {
        self.value.clone()
    }
    
    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }
}

/// 创建简单测试对象
#[cfg(feature = "python-bindings")]
#[pyfunction]
pub fn create_simple_test(value: String) -> PyResult<PySimpleTest> {
    Ok(PySimpleTest::new(value))
}

/// Python 模块注册函数
#[cfg(feature = "python-bindings")]
pub fn register_python_module(_py: Python, m: &PyModule) -> PyResult<()> {
    // 添加版本信息
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    // 注册简单测试类
    m.add_class::<PySimpleTest>()?;
    
    // 注册创建函数
    m.add_function(wrap_pyfunction!(create_simple_test, m)?)?;
    
    // 添加简化队列桥接器
    m.add_class::<PySimpleQueueBridge>()?;
    m.add_function(wrap_pyfunction!(create_simple_queue_bridge, m)?)?;
    
    Ok(())
}

/// Python 模块初始化
#[cfg(feature = "python-bindings")]
#[pymodule]
fn rat_quickdb(_py: Python, m: &PyModule) -> PyResult<()> {
    // 添加版本信息
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    // 注册简单测试类
    m.add_class::<PySimpleTest>()?;
    
    // 注册工厂函数
    m.add_function(wrap_pyfunction!(create_simple_test, m)?)?;
    
    Ok(())
}