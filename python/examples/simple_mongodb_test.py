#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
简化的MongoDB测试脚本
用于验证数据插入和查询功能
"""

import json
import time
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

def test_mongodb_basic():
    """测试MongoDB基本功能"""
    print("🚀 开始MongoDB基本功能测试")
    
    # 创建桥接器
    bridge = create_db_queue_bridge()
    
    try:
        # 创建缓存配置
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # L1缓存配置
        l1_config = PyL1CacheConfig(100)
        l1_config.max_memory_mb = 10
        l1_config.enable_stats = True
        cache_config.l1_config = l1_config
        
        # L2缓存配置
        l2_config = PyL2CacheConfig("./simple_test_cache")
        l2_config.max_disk_mb = 50
        l2_config.compression_level = 3
        l2_config.enable_wal = True
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
        
        # 添加MongoDB数据库连接
        print("📡 连接到MongoDB...")
        result = bridge.add_mongodb_database(
            alias="test_mongo",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=2,
            min_connections=1,
            connection_timeout=10,
            idle_timeout=30,
            max_lifetime=120,
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        
        print(f"连接结果: {result}")
        result_data = json.loads(result)
        if not result_data.get("success"):
            print(f"❌ 连接失败: {result_data.get('error')}")
            return
        
        print("✅ MongoDB连接成功")
        
        # 设置默认别名
        bridge.set_default_alias("test_mongo")
        
        # 清理可能存在的测试数据
        collection_name = f"simple_test_{int(time.time())}"
        print(f"🧹 使用集合: {collection_name}")
        
        # 插入测试数据
        print("📝 插入测试数据...")
        test_data = {
            "id": "user_001",
            "name": "张三",
            "age": 25,
            "city": "北京",
            "email": "zhangsan@example.com"
        }
        
        # 注意：create方法需要JSON字符串，但不要对字符串值进行双重编码
        create_result = bridge.create(collection_name, json.dumps(test_data, ensure_ascii=False))
        print(f"插入结果: {create_result}")
        
        create_data = json.loads(create_result)
        if not create_data.get("success"):
            print(f"❌ 插入失败: {create_data.get('error')}")
            return
        
        print("✅ 数据插入成功")
        
        # 等待一下确保数据已写入
        time.sleep(1)
        
        # 测试查询 - 查询所有数据
        print("🔍 查询所有数据...")
        query_result = bridge.find(collection_name, "[]")
        print(f"查询结果: {query_result}")
        
        query_data = json.loads(query_result)
        if query_data.get("success"):
            results = query_data.get("data", [])
            print(f"✅ 查询成功，找到 {len(results)} 条记录")
            for i, record in enumerate(results):
                print(f"  记录 {i+1}: {record}")
        else:
            print(f"❌ 查询失败: {query_data.get('error')}")
        
        # 测试条件查询 - 按name查询
        print("🔍 按name条件查询...")
        name_query = json.dumps({
            "field": "name",
            "operator": "Eq", 
            "value": "张三"
        })
        
        name_result = bridge.find(collection_name, name_query)
        print(f"按name查询结果: {name_result}")
        
        name_data = json.loads(name_result)
        if name_data.get("success"):
            results = name_data.get("data", [])
            print(f"✅ 按name查询成功，找到 {len(results)} 条记录")
            for i, record in enumerate(results):
                print(f"  记录 {i+1}: {record}")
        else:
            print(f"❌ 按name查询失败: {name_data.get('error')}")
        
        # 测试条件查询 - 按age查询
        print("🔍 按age条件查询...")
        age_query = json.dumps({
            "field": "age",
            "operator": "Eq", 
            "value": 25
        })
        
        age_result = bridge.find(collection_name, age_query)
        print(f"按age查询结果: {age_result}")
        
        age_data = json.loads(age_result)
        if age_data.get("success"):
            results = age_data.get("data", [])
            print(f"✅ 按age查询成功，找到 {len(results)} 条记录")
            for i, record in enumerate(results):
                print(f"  记录 {i+1}: {record}")
        else:
            print(f"❌ 按age查询失败: {age_data.get('error')}")
        
        # 清理测试数据
        print("🧹 清理测试数据...")
        try:
            bridge.drop_table(collection_name)
            print("✅ 测试数据清理完成")
        except Exception as e:
            print(f"⚠️ 清理测试数据时出错: {e}")
        
        print("🎉 MongoDB基本功能测试完成")
        
    except Exception as e:
        print(f"❌ 测试过程中出错: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_mongodb_basic()