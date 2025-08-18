#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
测试缓存禁用功能
验证当cache_config=None时，是否真的没有创建缓存管理器
"""

import sys
import os
import time
import json
from rat_quickdb_py import (
    create_db_queue_bridge,
    PyCacheConfig, PyL1CacheConfig, PyL2CacheConfig, 
    PyTtlConfig, PyCompressionConfig, PyTlsConfig, PyZstdConfig
)

def test_cache_disable():
    """测试缓存禁用功能"""
    print("🧪 开始测试缓存禁用功能...")
    
    # 创建数据库桥接器
    bridge = create_db_queue_bridge()
    
    # 测试1: 添加带缓存的数据库
    print("\n📊 测试1: 添加带缓存的数据库")
    cache_config = PyCacheConfig()
    cache_config.enable()
    cache_config.strategy = "lru"
    
    l1_config = PyL1CacheConfig(1000)
    l1_config.max_memory_mb = 100
    l1_config.enable_stats = True
    cache_config.l1_config = l1_config
    
    tls_config = PyTlsConfig()
    tls_config.enable()
    
    try:
        response = bridge.add_mongodb_database(
            alias="test_cached",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=5,
            min_connections=1,
            connection_timeout=5,
            idle_timeout=60,
            max_lifetime=300,
            cache_config=cache_config,  # 启用缓存
            tls_config=tls_config,
            zstd_config=None
        )
        result = json.loads(response)
        if result.get("success"):
            print("  ✅ 带缓存数据库添加成功")
        else:
            print(f"  ❌ 带缓存数据库添加失败: {result.get('error')}")
    except Exception as e:
        print(f"  ❌ 带缓存数据库添加异常: {e}")
    
    # 测试2: 添加不带缓存的数据库
    print("\n📊 测试2: 添加不带缓存的数据库")
    try:
        response = bridge.add_mongodb_database(
            alias="test_non_cached",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=5,
            min_connections=1,
            connection_timeout=5,
            idle_timeout=60,
            max_lifetime=300,
            cache_config=None,  # 不使用缓存
            tls_config=tls_config,
            zstd_config=None
        )
        result = json.loads(response)
        if result.get("success"):
            print("  ✅ 不带缓存数据库添加成功")
        else:
            print(f"  ❌ 不带缓存数据库添加失败: {result.get('error')}")
    except Exception as e:
        print(f"  ❌ 不带缓存数据库添加异常: {e}")
    
    # 测试3: 简单的查询性能对比
    print("\n📊 测试3: 简单查询性能对比")
    
    # 创建测试数据
    test_data = {
        "_id": "test_user_001",
        "name": "测试用户",
        "age": 25,
        "email": "test@example.com"
    }
    
    try:
        # 在两个数据库中都插入相同的测试数据
        bridge.create("test_users", json.dumps(test_data), "test_cached")
        bridge.create("test_users", json.dumps(test_data), "test_non_cached")
        print("  ✅ 测试数据插入成功")
        
        # 查询性能对比
        query_conditions = json.dumps([{"field": "name", "operator": "eq", "value": "测试用户"}])
        
        # 缓存数据库查询
        start_time = time.time()
        for i in range(10):
            bridge.find("test_users", query_conditions, "test_cached")
        cached_duration = (time.time() - start_time) * 1000
        
        # 非缓存数据库查询
        start_time = time.time()
        for i in range(10):
            bridge.find("test_users", query_conditions, "test_non_cached")
        non_cached_duration = (time.time() - start_time) * 1000
        
        print(f"  📈 缓存查询时间: {cached_duration:.2f}ms")
        print(f"  📈 非缓存查询时间: {non_cached_duration:.2f}ms")
        print(f"  📈 性能差异: {non_cached_duration/cached_duration:.2f}x")
        
        # 清理测试数据
        bridge.delete_by_id("test_users", "test_user_001", "test_cached")
        bridge.delete_by_id("test_users", "test_user_001", "test_non_cached")
        print("  ✅ 测试数据清理完成")
        
    except Exception as e:
        print(f"  ❌ 查询测试异常: {e}")
    
    print("\n🎉 缓存禁用功能测试完成")

if __name__ == "__main__":
    test_cache_disable()