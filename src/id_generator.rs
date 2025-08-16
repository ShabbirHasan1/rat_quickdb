//! ID 生成器模块
//! 
//! 提供多种 ID 生成策略，包括 UUID、雪花算法、MongoDB ObjectId 等。

use crate::types::{IdStrategy, IdType};
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zerg_creep::{info, warn, error};

/// ID 生成器
#[derive(Clone, Debug)]
pub struct IdGenerator {
    /// ID 生成策略
    strategy: IdStrategy,
    /// 雪花算法生成器
    snowflake_generator: Option<Arc<SnowflakeGenerator>>,
    /// 自增计数器（用于测试或简单场景）
    auto_increment_counter: Arc<AtomicU64>,
}

impl IdGenerator {
    /// 创建新的 ID 生成器
    pub fn new(strategy: IdStrategy) -> Result<Self> {
        let snowflake_generator = match &strategy {
            IdStrategy::Snowflake { machine_id, datacenter_id } => {
                Some(Arc::new(SnowflakeGenerator::new(*machine_id, *datacenter_id)?))
            }
            _ => None,
        };

        Ok(Self {
            strategy,
            snowflake_generator,
            auto_increment_counter: Arc::new(AtomicU64::new(1)),
        })
    }

    /// 生成新的 ID
    pub async fn generate(&self) -> Result<IdType> {
        match &self.strategy {
            IdStrategy::AutoIncrement => {
                let id = self.auto_increment_counter.fetch_add(1, Ordering::SeqCst);
                Ok(IdType::Number(id as i64))
            }
            IdStrategy::Uuid => {
                let uuid = Uuid::new_v4();
                Ok(IdType::String(uuid.to_string()))
            }
            IdStrategy::Snowflake { .. } => {
                if let Some(generator) = &self.snowflake_generator {
                    let id = generator.generate().await?;
                    Ok(IdType::String(id.to_string()))
                } else {
                    Err(anyhow!("Snowflake generator not initialized"))
                }
            }
            IdStrategy::ObjectId => {
                // 生成类似 MongoDB ObjectId 的字符串
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as u32;
                
                let random_bytes: [u8; 8] = rand::random();
                let object_id = format!(
                    "{:08x}{:016x}",
                    timestamp,
                    u64::from_be_bytes(random_bytes)
                );
                
                Ok(IdType::String(object_id))
            }
            IdStrategy::Custom(generator_name) => {
                // 这里可以根据自定义生成器名称来调用不同的生成逻辑
                // 目前简单返回一个基于时间戳的 ID
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                
                let custom_id = format!("{}_{}", generator_name, timestamp);
                Ok(IdType::String(custom_id))
            }
        }
    }

    /// 验证 ID 格式是否有效
    pub fn validate_id(&self, id: &IdType) -> bool {
        match (&self.strategy, id) {
            (IdStrategy::AutoIncrement, IdType::Number(n)) => *n > 0,
            (IdStrategy::Uuid, IdType::String(s)) => Uuid::parse_str(s).is_ok(),
            (IdStrategy::Snowflake { .. }, IdType::String(s)) => {
                s.parse::<u64>().is_ok()
            }
            (IdStrategy::ObjectId, IdType::String(s)) => {
                s.len() == 24 && s.chars().all(|c| c.is_ascii_hexdigit())
            }
            (IdStrategy::Custom(_), IdType::String(_)) => true, // 自定义策略不验证
            _ => false,
        }
    }

    /// 获取当前策略
    pub fn strategy(&self) -> &IdStrategy {
        &self.strategy
    }

    /// 设置自增计数器的起始值（仅用于 AutoIncrement 策略）
    pub fn set_auto_increment_start(&self, start: u64) {
        self.auto_increment_counter.store(start, Ordering::SeqCst);
    }
}

/// 雪花算法生成器
#[derive(Debug)]
struct SnowflakeGenerator {
    /// 机器 ID（10 位）
    machine_id: u16,
    /// 数据中心 ID（5 位）
    datacenter_id: u8,
    /// 序列号（12 位）
    sequence: Arc<AtomicU64>,
    /// 上次时间戳
    last_timestamp: Arc<AtomicU64>,
}

impl SnowflakeGenerator {
    /// 创建新的雪花算法生成器
    fn new(machine_id: u16, datacenter_id: u8) -> Result<Self> {
        if machine_id > 1023 {
            return Err(anyhow!("Machine ID must be between 0 and 1023"));
        }
        if datacenter_id > 31 {
            return Err(anyhow!("Datacenter ID must be between 0 and 31"));
        }

        Ok(Self {
            machine_id,
            datacenter_id,
            sequence: Arc::new(AtomicU64::new(0)),
            last_timestamp: Arc::new(AtomicU64::new(0)),
        })
    }

    /// 生成雪花算法 ID
    async fn generate(&self) -> Result<u64> {
        let mut timestamp = self.current_timestamp();
        let last_timestamp = self.last_timestamp.load(Ordering::SeqCst);

        if timestamp < last_timestamp {
            return Err(anyhow!("Clock moved backwards"));
        }

        let sequence = if timestamp == last_timestamp {
            let seq = self.sequence.fetch_add(1, Ordering::SeqCst) & 0xFFF;
            if seq == 0 {
                // 序列号溢出，等待下一毫秒
                timestamp = self.wait_next_millis(last_timestamp);
            }
            seq
        } else {
            self.sequence.store(0, Ordering::SeqCst);
            0
        };

        self.last_timestamp.store(timestamp, Ordering::SeqCst);

        // 组装雪花算法 ID
        // 时间戳（41位） + 数据中心ID（5位） + 机器ID（10位） + 序列号（12位）
        let id = ((timestamp - 1288834974657) << 22)
            | ((self.datacenter_id as u64) << 17)
            | ((self.machine_id as u64) << 12)
            | sequence;

        Ok(id)
    }

    /// 获取当前时间戳（毫秒）
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// 等待下一毫秒
    fn wait_next_millis(&self, last_timestamp: u64) -> u64 {
        let mut timestamp = self.current_timestamp();
        while timestamp <= last_timestamp {
            timestamp = self.current_timestamp();
        }
        timestamp
    }
}

/// MongoDB 自增 ID 生成器
/// 
/// 用于在 MongoDB 中实现类似 MySQL 的自增 ID 功能
#[derive(Debug)]
pub struct MongoAutoIncrementGenerator {
    /// 集合名称
    collection_name: String,
    /// 当前计数器值
    counter: Arc<AtomicU64>,
}

impl MongoAutoIncrementGenerator {
    /// 创建新的 MongoDB 自增 ID 生成器
    pub fn new(collection_name: String) -> Self {
        Self {
            collection_name,
            counter: Arc::new(AtomicU64::new(1)),
        }
    }

    /// 生成下一个自增 ID
    pub async fn next_id(&self) -> Result<i64> {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(id as i64)
    }

    /// 设置计数器起始值
    pub fn set_start_value(&self, start: u64) {
        self.counter.store(start, Ordering::SeqCst);
    }

    /// 获取当前计数器值
    pub fn current_value(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }

    /// 重置计数器
    pub fn reset(&self) {
        self.counter.store(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_uuid_generator() {
        let generator = IdGenerator::new(IdStrategy::Uuid).unwrap();
        
        let id1 = generator.generate().await.unwrap();
        let id2 = generator.generate().await.unwrap();
        
        // UUID 应该是字符串类型且不相等
        match (&id1, &id2) {
            (IdType::String(s1), IdType::String(s2)) => {
                assert_ne!(s1, s2);
                assert!(generator.validate_id(&id1));
                assert!(generator.validate_id(&id2));
            }
            _ => panic!("Expected string IDs"),
        }
    }

    #[tokio::test]
    async fn test_auto_increment_generator() {
        let generator = IdGenerator::new(IdStrategy::AutoIncrement).unwrap();
        
        let id1 = generator.generate().await.unwrap();
        let id2 = generator.generate().await.unwrap();
        
        // 自增 ID 应该是数字类型且递增
        match (&id1, &id2) {
            (IdType::Number(n1), IdType::Number(n2)) => {
                assert_eq!(*n1, 1);
                assert_eq!(*n2, 2);
                assert!(generator.validate_id(&id1));
                assert!(generator.validate_id(&id2));
            }
            _ => panic!("Expected number IDs"),
        }
    }

    #[tokio::test]
    async fn test_snowflake_generator() {
        let generator = IdGenerator::new(IdStrategy::Snowflake {
            machine_id: 1,
            datacenter_id: 1,
        }).unwrap();
        
        let id1 = generator.generate().await.unwrap();
        let id2 = generator.generate().await.unwrap();
        
        // 雪花算法 ID 应该是字符串类型且不相等
        match (&id1, &id2) {
            (IdType::String(s1), IdType::String(s2)) => {
                assert_ne!(s1, s2);
                assert!(generator.validate_id(&id1));
                assert!(generator.validate_id(&id2));
            }
            _ => panic!("Expected string IDs"),
        }
    }

    #[tokio::test]
    async fn test_object_id_generator() {
        let generator = IdGenerator::new(IdStrategy::ObjectId).unwrap();
        
        let id1 = generator.generate().await.unwrap();
        let id2 = generator.generate().await.unwrap();
        
        // ObjectId 应该是字符串类型且不相等
        match (&id1, &id2) {
            (IdType::String(s1), IdType::String(s2)) => {
                assert_ne!(s1, s2);
                assert_eq!(s1.len(), 24);
                assert_eq!(s2.len(), 24);
                assert!(generator.validate_id(&id1));
                assert!(generator.validate_id(&id2));
            }
            _ => panic!("Expected string IDs"),
        }
    }

    #[tokio::test]
    async fn test_mongo_auto_increment_generator() {
        let generator = MongoAutoIncrementGenerator::new("test_collection".to_string());
        
        let id1 = generator.next_id().await.unwrap();
        let id2 = generator.next_id().await.unwrap();
        
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        
        // 测试设置起始值
        generator.set_start_value(100);
        let id3 = generator.next_id().await.unwrap();
        assert_eq!(id3, 100);
    }
}