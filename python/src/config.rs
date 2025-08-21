use pyo3::prelude::*;
use rat_quickdb::types::{
    CacheConfig, CacheStrategy, L1CacheConfig, L2CacheConfig, TtlConfig, 
    CompressionConfig, CompressionAlgorithm, TlsConfig, ZstdConfig,
};

/// 缓存配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyCacheConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub strategy: String,
    #[pyo3(get, set)]
    pub l1_config: Option<PyL1CacheConfig>,
    #[pyo3(get, set)]
    pub l2_config: Option<PyL2CacheConfig>,
    #[pyo3(get, set)]
    pub ttl_config: Option<PyTtlConfig>,
    #[pyo3(get, set)]
    pub compression_config: Option<PyCompressionConfig>,
}

/// L1缓存配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyL1CacheConfig {
    #[pyo3(get, set)]
    pub max_capacity: usize,
    #[pyo3(get, set)]
    pub max_memory_mb: usize,
    #[pyo3(get, set)]
    pub enable_stats: bool,
}

/// L2缓存配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyL2CacheConfig {
    #[pyo3(get, set)]
    pub storage_path: String,
    #[pyo3(get, set)]
    pub max_disk_mb: usize,
    #[pyo3(get, set)]
    pub compression_level: i32,
    #[pyo3(get, set)]
    pub enable_wal: bool,
    #[pyo3(get, set)]
    pub clear_on_startup: bool,
}

/// TTL配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyTtlConfig {
    #[pyo3(get, set)]
    pub default_ttl_secs: u64,
    #[pyo3(get, set)]
    pub max_ttl_secs: u64,
    #[pyo3(get, set)]
    pub check_interval_secs: u64,
}

/// 压缩配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyCompressionConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub algorithm: String,
    #[pyo3(get, set)]
    pub threshold_bytes: usize,
}

/// TLS配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyTlsConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub ca_cert_path: Option<String>,
    #[pyo3(get, set)]
    pub client_cert_path: Option<String>,
    #[pyo3(get, set)]
    pub client_key_path: Option<String>,
    #[pyo3(get, set)]
    pub verify_server_cert: bool,
    #[pyo3(get, set)]
    pub verify_hostname: bool,
    #[pyo3(get, set)]
    pub min_tls_version: Option<String>,
    #[pyo3(get, set)]
    pub cipher_suites: Option<Vec<String>>,
}

/// ZSTD压缩配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyZstdConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub compression_level: Option<i32>,
    #[pyo3(get, set)]
    pub compression_threshold: Option<usize>,
}

#[pymethods]
impl PyCacheConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            enabled: false,
            strategy: "L1Only".to_string(),
            l1_config: None,
            l2_config: None,
            ttl_config: None,
            compression_config: None,
        }
    }

    /// 启用缓存
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用缓存
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

#[pymethods]
impl PyL1CacheConfig {
    #[new]
    pub fn new(max_capacity: usize) -> Self {
        Self {
            max_capacity,
            max_memory_mb: 100,
            enable_stats: false,
        }
    }
}

#[pymethods]
impl PyL2CacheConfig {
    #[new]
    pub fn new(storage_path: String) -> Self {
        Self {
            storage_path,
            max_disk_mb: 1000,
            compression_level: 3,
            enable_wal: true,
            clear_on_startup: false,
        }
    }
}

#[pymethods]
impl PyTtlConfig {
    #[new]
    pub fn new(default_ttl_secs: u64) -> Self {
        Self {
            default_ttl_secs,
            max_ttl_secs: default_ttl_secs * 2,
            check_interval_secs: 300,
        }
    }
}

#[pymethods]
impl PyCompressionConfig {
    #[new]
    pub fn new(algorithm: String) -> Self {
        Self {
            enabled: false,
            algorithm,
            threshold_bytes: 1024,
        }
    }
}

#[pymethods]
impl PyTlsConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            enabled: false,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            verify_server_cert: true,
            verify_hostname: true,
            min_tls_version: Some("1.2".to_string()),
            cipher_suites: None,
        }
    }
    
    /// 启用TLS
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// 禁用TLS
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

#[pymethods]
impl PyZstdConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            enabled: false,
            compression_level: Some(3),
            compression_threshold: Some(1024),
        }
    }
    
    /// 启用ZSTD压缩
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// 禁用ZSTD压缩
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

impl PyTlsConfig {
    /// 转换为Rust的TlsConfig
    pub fn to_rust_config(&self) -> TlsConfig {
        TlsConfig {
            enabled: self.enabled,
            ca_cert_path: self.ca_cert_path.clone(),
            client_cert_path: self.client_cert_path.clone(),
            client_key_path: self.client_key_path.clone(),
            verify_server_cert: self.verify_server_cert,
            verify_hostname: self.verify_hostname,
            min_tls_version: self.min_tls_version.clone(),
            cipher_suites: self.cipher_suites.clone(),
        }
    }
}

impl PyZstdConfig {
    /// 转换为Rust的ZstdConfig
    pub fn to_rust_config(&self) -> ZstdConfig {
        ZstdConfig {
            enabled: self.enabled,
            compression_level: self.compression_level,
            compression_threshold: self.compression_threshold,
        }
    }
}

impl PyCacheConfig {
    /// 转换为Rust的CacheConfig
    pub fn to_rust_config(&self) -> CacheConfig {
        let mut config = CacheConfig::default();
        config.enabled = self.enabled;
        
        // 设置缓存策略
        config.strategy = match self.strategy.as_str() {
            "lru" => CacheStrategy::Lru,
            "lfu" => CacheStrategy::Lfu,
            "fifo" => CacheStrategy::Fifo,
            _ => CacheStrategy::Lru,
        };
        
        // 设置L1缓存配置（必需字段）
        config.l1_config = if let Some(ref l1_config) = self.l1_config {
            L1CacheConfig {
                max_capacity: l1_config.max_capacity,
                max_memory_mb: l1_config.max_memory_mb,
                enable_stats: l1_config.enable_stats,
            }
        } else {
            L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: false,
            }
        };
        
        // 设置L2缓存配置（可选字段）
        config.l2_config = if let Some(ref l2_config) = self.l2_config {
            Some(L2CacheConfig {
                storage_path: l2_config.storage_path.clone(),
                max_disk_mb: l2_config.max_disk_mb,
                compression_level: l2_config.compression_level,
                enable_wal: l2_config.enable_wal,
                clear_on_startup: l2_config.clear_on_startup,
            })
        } else {
            None
        };
        
        // 设置TTL配置（必需字段）
        config.ttl_config = if let Some(ref ttl_config) = self.ttl_config {
            TtlConfig {
                default_ttl_secs: ttl_config.default_ttl_secs,
                max_ttl_secs: ttl_config.max_ttl_secs,
                check_interval_secs: ttl_config.check_interval_secs,
            }
        } else {
            TtlConfig {
                default_ttl_secs: 3600,
                max_ttl_secs: 7200,
                check_interval_secs: 300,
            }
        };
        
        // 设置压缩配置（必需字段）
        config.compression_config = if let Some(ref compression_config) = self.compression_config {
            CompressionConfig {
                enabled: compression_config.enabled,
                algorithm: match compression_config.algorithm.as_str() {
                    "gzip" => CompressionAlgorithm::Gzip,
                    "zstd" => CompressionAlgorithm::Zstd,
                    "lz4" => CompressionAlgorithm::Lz4,
                    _ => CompressionAlgorithm::Gzip,
                },
                threshold_bytes: compression_config.threshold_bytes,
            }
        } else {
            CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::Gzip,
                threshold_bytes: 1024,
            }
        };
        
        config
    }
}