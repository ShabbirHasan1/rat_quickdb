#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MongoDB调试测试脚本
用于检查数据插入和查询是否正常工作
"""

import json
import os
import time
from datetime import datetime
from rat_quickdb_py import (
    create_db_queue_bridge, 
    PyCacheConfig, 
    PyL1CacheConfig,
    PyL2CacheConfig,
    PyTtlConfig,
    PyCompressionConfig,
    PyTlsConfig,
    PyZstdConfig
)

def test_mongodb_basic_operations():
    """测试MongoDB基本操作"""
    print("🔍 开始MongoDB基本操作测试...")
    
    # 创建桥接器
    bridge = create_db_queue_bridge()
    cache_dir = "./test_mongodb_debug"
    
    # 创建缓存目录
    os.makedirs(cache_dir, exist_ok=True)
    
    try:
        # 创建简单的缓存配置
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # L1缓存配置
        l1_config = PyL1CacheConfig(100)
        l1_config.max_memory_mb = 10
        l1_config.enable_stats = True
        cache_config.l1_config = l1_config
        
        # L2缓存配置
        l2_config = PyL2CacheConfig(cache_dir)
        l2_config.max_disk_mb = 50
        l2_config.compression_level = 3
        l2_config.enable_wal = False
        l2_config.clear_on_startup = True
        cache_config.l2_config = l2_config
        
        # TTL配置
        ttl_config = PyTtlConfig(300)
        ttl_config.max_ttl_secs = 600
        ttl_config.check_interval_secs = 60
        cache_config.ttl_config = ttl_config
        
        # 压缩配置
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = True
        compression_config.threshold_bytes = 512
        cache_config.compression_config = compression_config
        
        # TLS配置
        tls_config = PyTlsConfig()
        tls_config.enable()
        tls_config.ca_cert_path = "/etc/ssl/certs/ca-certificates.crt"
        tls_config.client_cert_path = ""
        tls_config.client_key_path = ""
        
        # ZSTD配置
        zstd_config = PyZstdConfig()
        zstd_config.enable()
        zstd_config.compression_level = 3
        zstd_config.compression_threshold = 1024
        
        # 添加MongoDB数据库
        print("📡 连接MongoDB数据库...")
        result = bridge.add_mongodb_database(
            alias="mongodb_debug",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=4,
            min_connections=1,
            connection_timeout=10,
            idle_timeout=120,
            max_lifetime=600,
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        print(f"MongoDB连接结果: {result}")
        
        # 设置默认别名
        bridge.set_default_alias("mongodb_debug")
        
        # 使用时间戳作为集合名
        timestamp = int(time.time() * 1000)
        collection_name = f"debug_test_{timestamp}"
        print(f"📝 使用集合名: {collection_name}")
        
        # 插入一条简单的测试数据
        test_data = {
            "_id": "test_001",
            "name": "测试用户",
            "age": 25,
            "city": "北京",
            "department": "技术部",
            "created_at": datetime.utcnow().isoformat() + "Z"
        }
        
        print("📝 插入测试数据...")
        test_data_json = json.dumps(test_data)
        insert_result = bridge.create(collection_name, test_data_json, "mongodb_debug")
        print(f"插入结果: {insert_result}")
        
        # 等待一下确保数据已插入
        time.sleep(1)
        
        # 测试简单查询
        print("\n🔍 测试简单查询...")
        
        # 1. 查询所有数据（空条件）
        print("1. 查询所有数据:")
        all_query = json.dumps([])
        all_result = bridge.find(collection_name, all_query, "mongodb_debug")
        print(f"   查询结果: {all_result}")
        
        # 2. 按_id查询
        print("2. 按_id查询:")
        id_query = json.dumps({
            "field": "_id",
            "operator": "Eq",
            "value": "test_001"
        })
        id_result = bridge.find(collection_name, id_query, "mongodb_debug")
        print(f"   查询结果: {id_result}")
        
        # 3. 按name查询
        print("3. 按name查询:")
        name_query = json.dumps({
            "field": "name",
            "operator": "Eq",
            "value": "测试用户"
        })
        name_result = bridge.find(collection_name, name_query, "mongodb_debug")
        print(f"   查询结果: {name_result}")
        
        # 4. 按age查询
        print("4. 按age查询:")
        age_query = json.dumps({
            "field": "age",
            "operator": "Eq",
            "value": 25
        })
        age_result = bridge.find(collection_name, age_query, "mongodb_debug")
        print(f"   查询结果: {age_result}")
        
        # 5. 使用数组格式查询
        print("5. 使用数组格式查询:")
        array_query = json.dumps([
            {"field": "department", "operator": "Eq", "value": "技术部"}
        ])
        array_result = bridge.find(collection_name, array_query, "mongodb_debug")
        print(f"   查询结果: {array_result}")
        
        # 6. 使用键值对格式查询
        print("6. 使用键值对格式查询:")
        kv_query = json.dumps({
            "city": "北京"
        })
        kv_result = bridge.find(collection_name, kv_query, "mongodb_debug")
        print(f"   查询结果: {kv_result}")
        
        # 清理测试数据
        print("\n🧹 清理测试数据...")
        delete_query = json.dumps({
            "field": "_id",
            "operator": "Eq",
            "value": "test_001"
        })
        delete_result = bridge.delete(collection_name, delete_query, "mongodb_debug")
        print(f"删除结果: {delete_result}")
        
        print("\n✅ MongoDB基本操作测试完成")
        
    except Exception as e:
        print(f"❌ 测试过程中出现错误: {e}")
        import traceback
        traceback.print_exc()
    finally:
        # 清理缓存目录
        try:
            import shutil
            if os.path.exists(cache_dir):
                shutil.rmtree(cache_dir)
                print(f"🗑️ 已清理缓存目录: {cache_dir}")
        except Exception as e:
            print(f"⚠️ 清理缓存目录失败: {e}")

if __name__ == "__main__":
    test_mongodb_basic_operations()