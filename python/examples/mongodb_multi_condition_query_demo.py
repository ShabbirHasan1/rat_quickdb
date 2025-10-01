#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MongoDB多条件查询演示

本示例展示了 rat_quickdb 在 MongoDB 环境下支持的多种查询条件格式：
1. 单个查询条件对象格式
2. 多个查询条件数组格式 (AND逻辑)
3. 简化的键值对格式
4. OR逻辑查询格式
5. MongoDB特有的复杂查询操作符

基于 SQLite 版本的多条件查询示例改写为 MongoDB 版本
"""

import json
import os
import time
import shutil
import signal
import threading
import sys
from datetime import datetime, timezone
from typing import Dict, List, Optional
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

# 导入优雅关闭机制
from graceful_shutdown import GracefulShutdownMixin, ShutdownConfig, with_graceful_shutdown

# 全局变量用于强制退出机制
shutdown_lock = threading.Lock()
shutdown_timeout = 15  # 强制退出超时时间（秒）
test_instance = None


def force_exit():
    """强制退出函数"""
    print(f"\n⚠️ 优雅关闭超时（{shutdown_timeout}秒），强制退出程序...")
    os._exit(1)


def signal_handler(signum, frame):
    """信号处理器，支持强制退出机制"""
    global test_instance
    
    with shutdown_lock:
        print(f"\n🛑 收到信号 {signum}，开始优雅关闭...")
        
        # 启动强制退出定时器
        force_exit_timer = threading.Timer(shutdown_timeout, force_exit)
        force_exit_timer.daemon = True
        force_exit_timer.start()
        
        try:
            if test_instance:
                test_instance.shutdown()
        except Exception as e:
            print(f"⚠️ 优雅关闭过程中出错: {e}")
        finally:
            force_exit_timer.cancel()
            print("✅ 优雅关闭完成")
            sys.exit(0)


class MongoDbMultiConditionQueryDemo(GracefulShutdownMixin):
    def __init__(self):
        # 初始化优雅关闭机制
        super().__init__(ShutdownConfig(
            shutdown_timeout=10,  # 减少关闭超时时间到10秒
            verbose_logging=True,
            auto_cleanup_on_exit=True
        ))
        
        self.bridge = create_db_queue_bridge()
        self.cache_dir = "./mongodb_multi_query_cache"
        self.add_temp_dir(self.cache_dir)
        
        # 使用时间戳作为集合名后缀，避免重复
        timestamp = int(time.time() * 1000)
        self.collection_name = f"demo_users_{timestamp}"
        
    def _cleanup_existing_collections(self):
        """清理现有的测试集合"""
        print("🧹 清理现有的测试集合...")
        try:
            # 创建临时桥接器进行清理
            temp_bridge = create_db_queue_bridge()
            
            # 添加数据库连接
            response = temp_bridge.add_mongodb_database(
                alias="mongodb_cleanup",
                host="db0.0ldm0s.net",
                port=27017,
                database="testdb",
                username="testdb",
                password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
                auth_source="testdb",
                direct_connection=True,
                max_connections=2,
                min_connections=1,
                connection_timeout=5,
                idle_timeout=30,
                max_lifetime=120
            )
            
            result = json.loads(response)
            if result.get("success"):
                # 清理可能存在的测试集合
                collections_to_clean = ["demo_users", "test_users", "users", self.collection_name]
                for collection in collections_to_clean:
                    try:
                        temp_bridge.drop_table(collection, "mongodb_cleanup")
                        print(f"✅ 已清理集合: {collection}")
                    except Exception as e:
                        print(f"⚠️ 清理集合 {collection} 时出错: {e}")
            else:
                print(f"⚠️ 无法连接到MongoDB进行清理: {result.get('error')}")
                
        except Exception as e:
            print(f"⚠️ 清理现有集合时出错: {e}")
        
    def setup_database(self):
        """设置MongoDB数据库和测试数据"""
        print("🔧 设置MongoDB数据库...")
        
        # 创建缓存目录
        os.makedirs(self.cache_dir, exist_ok=True)
        
        # 创建MongoDB缓存配置
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # L1缓存配置
        l1_config = PyL1CacheConfig(800)  # 最大容量800条记录
        l1_config.max_memory_mb = 80  # 最大内存80MB
        l1_config.enable_stats = True  # 启用统计
        cache_config.l1_config = l1_config
        
        # L2缓存配置
        l2_config = PyL2CacheConfig(self.cache_dir)
        l2_config.max_disk_mb = 300  # 最大磁盘300MB
        l2_config.compression_level = 6
        l2_config.enable_wal = True
        l2_config.clear_on_startup = False  # 启动时不清空缓存目录
        cache_config.l2_config = l2_config
        
        # TTL配置
        ttl_config = PyTtlConfig(450)  # 默认TTL 7.5分钟
        ttl_config.max_ttl_secs = 1800  # 最大TTL 30分钟
        ttl_config.check_interval_secs = 90  # 检查间隔1.5分钟
        cache_config.ttl_config = ttl_config
        
        # 压缩配置
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = True
        compression_config.threshold_bytes = 768
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
        result = self.bridge.add_mongodb_database(
            alias="mongodb_demo",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=8,
            min_connections=2,
            connection_timeout=5,   # 减少连接超时时间到5秒
            idle_timeout=60,        # 减少空闲超时时间到1分钟
            max_lifetime=300,       # 减少最大生命周期到5分钟
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        print(f"MongoDB数据库添加结果: {result}")
        
        # 注册数据库连接到优雅关闭机制
        self.add_database_connection(self.bridge)
        
        # 设置默认别名
        self.bridge.set_default_alias("mongodb_demo")
        
        # 插入测试数据（MongoDB格式）
        test_users = [
            {
                "id": "user_001",
                "name": "张三", 
                "age": 25, 
                "city": "北京", 
                "department": "技术部", 
                "salary": 8000,
                "skills": ["Python", "MongoDB", "Docker"],
                "profile": {
                    "education": "本科",
                    "experience_years": 3,
                    "certification": ["AWS", "MongoDB"]
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
            {
                "id": "user_002",
                "name": "李四", 
                "age": 30, 
                "city": "上海", 
                "department": "销售部", 
                "salary": 12000,
                "skills": ["销售", "客户管理", "CRM"],
                "profile": {
                    "education": "硕士",
                    "experience_years": 6,
                    "certification": ["PMP"]
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
            {
                "id": "user_003",
                "name": "王五", 
                "age": 28, 
                "city": "广州", 
                "department": "技术部", 
                "salary": 9500,
                "skills": ["Java", "Spring", "MySQL"],
                "profile": {
                    "education": "本科",
                    "experience_years": 4,
                    "certification": ["Oracle", "Java"]
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
            {
                "id": "user_004",
                "name": "赵六", 
                "age": 35, 
                "city": "深圳", 
                "department": "市场部", 
                "salary": 15000,
                "skills": ["市场分析", "数据分析", "PowerBI"],
                "profile": {
                    "education": "硕士",
                    "experience_years": 8,
                    "certification": ["Google Analytics", "Facebook Marketing"]
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
            {
                "id": "user_005",
                "name": "钱七", 
                "age": 22, 
                "city": "杭州", 
                "department": "技术部", 
                "salary": 7000,
                "skills": ["JavaScript", "React", "Node.js"],
                "profile": {
                    "education": "本科",
                    "experience_years": 1,
                    "certification": []
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
            {
                "id": "user_006",
                "name": "孙八", 
                "age": 40, 
                "city": "成都", 
                "department": "管理部", 
                "salary": 20000,
                "skills": ["团队管理", "项目管理", "战略规划"],
                "profile": {
                    "education": "MBA",
                    "experience_years": 15,
                    "certification": ["PMP", "CISSP", "MBA"]
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
            {
                "id": "user_007",
                "name": "周九", 
                "age": 26, 
                "city": "西安", 
                "department": "销售部", 
                "salary": 8500,
                "skills": ["B2B销售", "谈判", "Salesforce"],
                "profile": {
                    "education": "本科",
                    "experience_years": 3,
                    "certification": ["Salesforce Admin"]
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
            {
                "id": "user_008",
                "name": "吴十", 
                "age": 33, 
                "city": "南京", 
                "department": "技术部", 
                "salary": 11000,
                "skills": ["DevOps", "Kubernetes", "AWS"],
                "profile": {
                    "education": "硕士",
                    "experience_years": 7,
                    "certification": ["AWS Solutions Architect", "CKA"]
                },
                "created_at": datetime.now(timezone.utc).isoformat(),
                "is_active": True
            },
        ]
        
        print(f"📝 插入MongoDB测试数据到集合 {self.collection_name}...")
        for user in test_users:
            user_json = json.dumps(user)
            result = self.bridge.create(self.collection_name, user_json, "mongodb_demo")
            print(f"插入用户 {user['name']}: {result}")
            
        # 验证数据是否成功插入
        print("\n🔍 验证数据插入情况...")
        verify_query = json.dumps({})
        verify_result = self.bridge.find(self.collection_name, verify_query, "mongodb_demo")
        print(f"数据验证查询结果: {verify_result}")
            
        print("✅ MongoDB数据库设置完成\n")
        print(f"🏷️  集合名称: {self.collection_name}")
        print(f"🌐 MongoDB主机: db0.0ldm0s.net:27017")
        print(f"🗄️  数据库: testdb")
        print(f"🔒 TLS: 启用")
        print(f"🗜️  ZSTD压缩: 启用")
        print(f"💾 缓存: 启用（L1+L2）\n")
        
    def demo_single_condition_query(self):
        """演示单个查询条件格式"""
        print("🔍 演示MongoDB单个查询条件格式")
        print("格式: {\"field\": \"字段名\", \"operator\": \"操作符\", \"value\": \"值\"}")
        
        # 示例1: 等值查询（MongoDB id查询）
        query1 = json.dumps({
            "field": "id", 
            "operator": "Eq", 
            "value": "user_001"
        })
        print(f"\n查询条件（MongoDB id查询）: {query1}")
        result1 = self.bridge.find(self.collection_name, query1, "mongodb_demo")
        print(f"查询结果: {result1}")
        
        # 示例2: 大于查询（年龄）
        query2 = json.dumps({
            "field": "age", 
            "operator": "Gt", 
            "value": 30
        })
        print(f"\n查询条件（年龄大于30）: {query2}")
        result2 = self.bridge.find(self.collection_name, query2, "mongodb_demo")
        print(f"查询结果: {result2}")
        
        # 示例3: 包含查询（城市名包含"京"）
        query3 = json.dumps({
            "field": "city", 
            "operator": "Contains", 
            "value": "京"
        })
        print(f"\n查询条件（城市包含\"京\"）: {query3}")
        result3 = self.bridge.find(self.collection_name, query3, "mongodb_demo")
        print(f"查询结果: {result3}")
        
        # 示例4: MongoDB嵌套文档查询
        query4 = json.dumps({
            "field": "profile.education", 
            "operator": "Eq", 
            "value": "硕士"
        })
        print(f"\n查询条件（嵌套文档-学历为硕士）: {query4}")
        result4 = self.bridge.find(self.collection_name, query4, "mongodb_demo")
        print(f"查询结果: {result4}")
        
        # 示例5: MongoDB数组字段查询
        query5 = json.dumps({
            "field": "skills", 
            "operator": "Contains", 
            "value": "Python"
        })
        print(f"\n查询条件（技能包含Python）: {query5}")
        result5 = self.bridge.find(self.collection_name, query5, "mongodb_demo")
        print(f"查询结果: {result5}")
        
    def demo_multi_condition_array_query(self):
        """演示多个查询条件数组格式"""
        print("\n\n🔍 演示MongoDB多个查询条件数组格式 (AND逻辑)")
        print("格式: [{\"field\": \"字段1\", \"operator\": \"操作符1\", \"value\": \"值1\"}, {\"field\": \"字段2\", \"operator\": \"操作符2\", \"value\": \"值2\"}]")
        
        # 示例1: 年龄大于25且部门为技术部
        query1 = json.dumps([
            {"field": "age", "operator": "Gt", "value": 25},
            {"field": "department", "operator": "Eq", "value": "技术部"}
        ])
        print(f"\n查询条件（年龄>25 AND 技术部）: {query1}")
        result1 = self.bridge.find(self.collection_name, query1, "mongodb_demo")
        print(f"查询结果: {result1}")
        
        # 示例2: 薪资在8000-12000之间且城市包含"海"或"京"
        query2 = json.dumps([
            {"field": "salary", "operator": "Gte", "value": 8000},
            {"field": "salary", "operator": "Lte", "value": 12000},
            {"field": "city", "operator": "Contains", "value": "海"}
        ])
        print(f"\n查询条件（薪资8000-12000 AND 城市包含\"海\"）: {query2}")
        result2 = self.bridge.find(self.collection_name, query2, "mongodb_demo")
        print(f"查询结果: {result2}")
        
        # 示例3: 复杂多条件查询（包含嵌套文档）
        query3 = json.dumps([
            {"field": "age", "operator": "Gte", "value": 25},
            {"field": "age", "operator": "Lt", "value": 35},
            {"field": "department", "operator": "Ne", "value": "管理部"},
            {"field": "salary", "operator": "Gt", "value": 7500},
            {"field": "profile.experience_years", "operator": "Gte", "value": 3}
        ])
        print(f"\n查询条件（复杂多条件+嵌套文档）: {query3}")
        result3 = self.bridge.find(self.collection_name, query3, "mongodb_demo")
        print(f"查询结果: {result3}")
        
        # 示例4: MongoDB数组字段多条件查询
        query4 = json.dumps([
            {"field": "skills", "operator": "Contains", "value": "Python"},
            {"field": "profile.certification", "operator": "Contains", "value": "AWS"}
        ])
        print(f"\n查询条件（技能包含Python AND 认证包含AWS）: {query4}")
        result4 = self.bridge.find(self.collection_name, query4, "mongodb_demo")
        print(f"查询结果: {result4}")
        
    def demo_simplified_key_value_query(self):
        """演示简化的键值对格式"""
        print("\n\n🔍 演示MongoDB简化的键值对格式 (默认使用Eq操作符)")
        print("格式: {\"字段1\": \"值1\", \"字段2\": \"值2\"}")
        
        # 示例1: 简单等值查询（MongoDB id）
        query1 = json.dumps({
            "id": "user_002"
        })
        print(f"\n查询条件（MongoDB id查询）: {query1}")
        result1 = self.bridge.find(self.collection_name, query1, "mongodb_demo")
        print(f"查询结果: {result1}")
        
        # 示例2: 多字段等值查询
        query2 = json.dumps({
            "department": "技术部",
            "city": "广州"
        })
        print(f"\n查询条件（部门=技术部 AND 城市=广州）: {query2}")
        result2 = self.bridge.find(self.collection_name, query2, "mongodb_demo")
        print(f"查询结果: {result2}")
        
        # 示例3: 混合数据类型查询
        query3 = json.dumps({
            "age": 30,
            "department": "销售部",
            "is_active": True
        })
        print(f"\n查询条件（年龄=30 AND 部门=销售部 AND 激活状态=true）: {query3}")
        result3 = self.bridge.find(self.collection_name, query3, "mongodb_demo")
        print(f"查询结果: {result3}")
        
        # 示例4: MongoDB嵌套文档简化查询
        query4 = json.dumps({
            "profile.education": "本科",
            "department": "技术部"
        })
        print(f"\n查询条件（嵌套文档-学历=本科 AND 部门=技术部）: {query4}")
        result4 = self.bridge.find(self.collection_name, query4, "mongodb_demo")
        print(f"查询结果: {result4}")
        
    def demo_or_logic_query(self):
        """演示OR逻辑查询"""
        print("\n\n🔍 演示MongoDB OR逻辑查询")
        print("格式: {\"operator\": \"or\", \"conditions\": [{条件1}, {条件2}, ...]}")
        
        # 示例1: 简单OR查询 - 年龄大于35或薪资大于15000
        query1 = json.dumps({
            "operator": "or",
            "conditions": [
                {"field": "age", "operator": "Gt", "value": 35},
                {"field": "salary", "operator": "Gt", "value": 15000}
            ]
        })
        print(f"\n查询条件（年龄>35 OR 薪资>15000）: {query1}")
        result1 = self.bridge.find_with_groups(self.collection_name, query1, "mongodb_demo")
        print(f"查询结果: {result1}")
        
        # 示例2: 复杂OR查询 - 技术部员工或城市在北京/上海的员工
        query2 = json.dumps({
            "operator": "or",
            "conditions": [
                {"field": "department", "operator": "Eq", "value": "技术部"},
                {
                    "operator": "or",
                    "conditions": [
                        {"field": "city", "operator": "Eq", "value": "北京"},
                        {"field": "city", "operator": "Eq", "value": "上海"}
                    ]
                }
            ]
        })
        print(f"\n查询条件（技术部 OR (北京 OR 上海)）: {query2}")
        result2 = self.bridge.find_with_groups(self.collection_name, query2, "mongodb_demo")
        print(f"查询结果: {result2}")
        
        # 示例3: 混合AND/OR查询 - (年龄25-30且技术部) 或 (薪资>12000且销售部)
        query3 = json.dumps({
            "operator": "or",
            "conditions": [
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "age", "operator": "Gte", "value": 25},
                        {"field": "age", "operator": "Lte", "value": 30},
                        {"field": "department", "operator": "Eq", "value": "技术部"}
                    ]
                },
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "salary", "operator": "Gt", "value": 12000},
                        {"field": "department", "operator": "Eq", "value": "销售部"}
                    ]
                }
            ]
        })
        print(f"\n查询条件（(年龄25-30 AND 技术部) OR (薪资>12000 AND 销售部)）: {query3}")
        result3 = self.bridge.find_with_groups(self.collection_name, query3, "mongodb_demo")
        print(f"查询结果: {result3}")
        
        # 示例4: MongoDB嵌套文档和数组的OR查询
        query4 = json.dumps({
            "operator": "or",
            "conditions": [
                {"field": "profile.education", "operator": "Eq", "value": "MBA"},
                {"field": "skills", "operator": "Contains", "value": "Python"},
                {"field": "profile.experience_years", "operator": "Gte", "value": 10}
            ]
        })
        print(f"\n查询条件（MBA学历 OR 技能包含Python OR 经验>=10年）: {query4}")
        result4 = self.bridge.find_with_groups(self.collection_name, query4, "mongodb_demo")
        print(f"查询结果: {result4}")
        
        # 示例5: 单个条件组合格式（MongoDB数组查询）
        query5 = json.dumps([
            {
                "operator": "or",
                "conditions": [
                    {"field": "skills", "operator": "Contains", "value": "Java"},
                    {"field": "skills", "operator": "Contains", "value": "Python"},
                    {"field": "skills", "operator": "Contains", "value": "JavaScript"}
                ]
            }
        ])
        print(f"\n查询条件（技能包含Java OR Python OR JavaScript）: {query5}")
        result5 = self.bridge.find_with_groups(self.collection_name, query5, "mongodb_demo")
        print(f"查询结果: {result5}")
        
    def demo_mongodb_specific_queries(self):
        """演示MongoDB特有的查询功能"""
        print("\n\n🔍 演示MongoDB特有的查询功能")
        
        # 示例1: 数组长度查询（如果支持）
        print("\n1. 数组字段查询:")
        query1 = json.dumps([
            {"field": "skills", "operator": "Contains", "value": "MongoDB"},
            {"field": "profile.certification", "operator": "Contains", "value": "AWS"}
        ])
        print(f"查询条件（技能包含MongoDB AND 认证包含AWS）: {query1}")
        result1 = self.bridge.find(self.collection_name, query1, "mongodb_demo")
        print(f"查询结果: {result1}")
        
        # 示例2: 嵌套文档复杂查询
        print("\n2. 嵌套文档复杂查询:")
        query2 = json.dumps({
            "operator": "or",
            "conditions": [
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "profile.education", "operator": "Eq", "value": "硕士"},
                        {"field": "profile.experience_years", "operator": "Gte", "value": 5}
                    ]
                },
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "profile.education", "operator": "Eq", "value": "MBA"},
                        {"field": "age", "operator": "Gte", "value": 35}
                    ]
                }
            ]
        })
        print(f"查询条件（(硕士学历 AND 经验>=5年) OR (MBA学历 AND 年龄>=35)）: {query2}")
        result2 = self.bridge.find_with_groups(self.collection_name, query2, "mongodb_demo")
        print(f"查询结果: {result2}")
        
        # 示例3: 多个数组字段的复合查询
        print("\n3. 多个数组字段的复合查询:")
        query3 = json.dumps({
            "operator": "and",
            "conditions": [
                {"field": "skills", "operator": "Contains", "value": "AWS"},
                {"field": "profile.certification", "operator": "Contains", "value": "AWS"},
                {"field": "department", "operator": "Eq", "value": "技术部"}
            ]
        })
        print(f"查询条件（技能包含AWS AND 认证包含AWS AND 技术部）: {query3}")
        result3 = self.bridge.find_with_groups(self.collection_name, query3, "mongodb_demo")
        print(f"查询结果: {result3}")
        
    def demo_performance_comparison(self):
        """演示MongoDB查询性能对比"""
        print("\n\n⚡ MongoDB查询性能对比")
        
        # 复杂查询条件（包含嵌套文档和数组）
        complex_query = json.dumps([
            {"field": "age", "operator": "Gte", "value": 25},
            {"field": "salary", "operator": "Gt", "value": 8000},
            {"field": "department", "operator": "Eq", "value": "技术部"},
            {"field": "profile.experience_years", "operator": "Gte", "value": 3},
            {"field": "skills", "operator": "Contains", "value": "Python"}
        ])
        
        # 第一次查询（冷启动）
        start_time = time.time()
        result1 = self.bridge.find(self.collection_name, complex_query, "mongodb_demo")
        first_query_time = (time.time() - start_time) * 1000
        
        # 第二次查询（缓存命中）
        start_time = time.time()
        result2 = self.bridge.find(self.collection_name, complex_query, "mongodb_demo")
        second_query_time = (time.time() - start_time) * 1000
        
        # 第三次查询（缓存命中）
        start_time = time.time()
        result3 = self.bridge.find(self.collection_name, complex_query, "mongodb_demo")
        third_query_time = (time.time() - start_time) * 1000
        
        print(f"复杂MongoDB查询条件: {complex_query}")
        print(f"第一次查询时间（冷启动）: {first_query_time:.2f}ms")
        print(f"第二次查询时间（缓存命中）: {second_query_time:.2f}ms")
        print(f"第三次查询时间（缓存命中）: {third_query_time:.2f}ms")
        print(f"缓存性能提升: {(first_query_time / second_query_time):.2f}x")
        print(f"查询结果: {result1}")
        
        # OR逻辑查询性能测试
        print("\n🔄 OR逻辑查询性能测试:")
        or_query = json.dumps({
            "operator": "or",
            "conditions": [
                {"field": "department", "operator": "Eq", "value": "技术部"},
                {"field": "salary", "operator": "Gt", "value": 15000},
                {"field": "profile.education", "operator": "Eq", "value": "MBA"}
            ]
        })
        
        # OR查询性能测试
        start_time = time.time()
        or_result1 = self.bridge.find_with_groups(self.collection_name, or_query, "mongodb_demo")
        or_first_time = (time.time() - start_time) * 1000
        
        start_time = time.time()
        or_result2 = self.bridge.find_with_groups(self.collection_name, or_query, "mongodb_demo")
        or_second_time = (time.time() - start_time) * 1000
        
        print(f"OR查询条件: {or_query}")
        print(f"OR查询第一次: {or_first_time:.2f}ms")
        print(f"OR查询第二次: {or_second_time:.2f}ms")
        print(f"OR查询性能提升: {(or_first_time / or_second_time):.2f}x")
        print(f"OR查询结果: {or_result1}")
        
    def cleanup_resources(self):
        """清理MongoDB资源（实现 GracefulShutdownMixin 的抽象方法）"""
        print("🧹 清理MongoDB资源...")
        
        def timeout_handler(signum, frame):
            raise TimeoutError("清理操作超时")
        
        # 设置5秒超时
        signal.signal(signal.SIGALRM, timeout_handler)
        signal.alarm(5)
        
        try:
            # 删除测试集合数据
            if self.bridge:
                delete_conditions = json.dumps([
                    {"field": "_id", "operator": "Contains", "value": "user_"}
                ])
                result = self.bridge.delete(self.collection_name, delete_conditions, "mongodb_demo")
                print(f"🗑️  删除MongoDB测试数据: {result}")
                
            print("✅ MongoDB资源清理完成")
        except TimeoutError:
            print("⚠️ 清理操作超时，跳过清理")
        except Exception as e:
            print(f"❌ 清理失败: {e}")
        finally:
            signal.alarm(0)  # 取消超时
    
    def cleanup(self):
        """兼容性方法，调用优雅关闭"""
        self.shutdown()
            
    def run_demo(self):
        """运行完整的MongoDB演示"""
        print("🚀 MongoDB多条件查询演示开始\n")
        
        try:
            # 清理现有的测试集合
            self._cleanup_existing_collections()
            
            self.setup_database()
            self.demo_single_condition_query()
            self.demo_multi_condition_array_query()
            self.demo_simplified_key_value_query()
            self.demo_or_logic_query()
            self.demo_mongodb_specific_queries()
            self.demo_performance_comparison()
            
            print("\n\n🎉 MongoDB演示完成！")
            print("\n📋 MongoDB查询特性总结:")
            print("1. 单个查询条件格式: 支持MongoDB所有操作符，适合复杂单条件查询")
            print("2. 多条件数组格式: 支持复杂的AND逻辑组合查询，包含嵌套文档")
            print("3. 简化键值对格式: 适合简单的等值查询，支持嵌套文档字段")
            print("4. OR逻辑查询格式: 支持复杂的OR/AND混合逻辑查询")
            print("5. MongoDB特有功能: 嵌套文档查询、数组字段查询、复合索引")
            print("6. 缓存优化: 所有查询格式都支持缓存，显著提升MongoDB查询性能")
            print("7. 网络优化: TLS加密和ZSTD压缩减少网络传输开销")
            
            print("\n🔧 MongoDB技术特点:")
            print("   • 无模式约束，灵活的文档结构")
            print("   • 原生支持嵌套文档和数组字段")
            print("   • 强大的查询操作符和聚合管道")
            print("   • 水平扩展和分片支持")
            print("   • 丰富的索引类型（单字段、复合、文本、地理空间）")
            print(f"   • 集合名称: {self.collection_name}")
            
        except Exception as e:
            print(f"❌ 演示过程中出现错误: {e}")
            import traceback
            traceback.print_exc()
        finally:
            self.cleanup()


@with_graceful_shutdown(ShutdownConfig(verbose_logging=True))
def main():
    """主函数"""
    global test_instance
    
    # 注册信号处理器
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    demo = MongoDbMultiConditionQueryDemo()
    test_instance = demo  # 设置全局实例用于信号处理
    
    try:
        demo.run_demo()
    except KeyboardInterrupt:
        print("\n🛑 演示被用户中断")
    except Exception as e:
        print(f"\n❌ 演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()
    finally:
        try:
            if demo:
                demo.cleanup()
        except Exception as e:
            print(f"⚠️ 清理过程中出错: {e}")

if __name__ == "__main__":
    main()