#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python MongoDB 缓存性能简化测试

简化版本的MongoDB缓存性能对比测试
- 只进行500次查询对比
- 验证缓存禁用功能（非缓存查询时间必须>200ms）
"""

import json
import time
import os
import sys
from datetime import datetime
from typing import Optional
from dataclasses import dataclass

try:
    import rat_quickdb_py
    from rat_quickdb_py import (
        create_db_queue_bridge,
        get_version,
        get_info,
        get_name,
        PyCacheConfig,
        PyL1CacheConfig,
        PyL2CacheConfig,
        PyTtlConfig,
        PyCompressionConfig,
        PyTlsConfig,
        PyZstdConfig,
    )
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    exit(1)


@dataclass
class TestUser:
    """测试用户数据结构"""
    id: str
    name: str
    email: str
    age: int
    created_at: str
    
    @classmethod
    def new(cls, user_id: str, name: str, email: str, age: int) -> 'TestUser':
        return cls(
            id=user_id,
            name=name,
            email=email,
            age=age,
            created_at=datetime.utcnow().isoformat() + "Z"
        )
    
    def to_json(self) -> str:
        """转换为JSON字符串"""
        return json.dumps({
            "_id": self.id,  # MongoDB使用_id作为主键
            "name": self.name,
            "email": self.email,
            "age": self.age,
            "created_at": self.created_at
        })


class MongoDbCacheSimpleTest:
    """MongoDB缓存性能简化测试"""
    
    @staticmethod
    def get_ca_cert_path():
        """获取跨平台的CA证书路径"""
        import platform
        
        system = platform.system().lower()
        
        if system == "darwin":  # macOS
            ca_paths = [
                "/etc/ssl/cert.pem",
                "/usr/local/etc/openssl/cert.pem",
                "/opt/homebrew/etc/openssl/cert.pem"
            ]
        elif system == "linux":
            ca_paths = [
                "/etc/ssl/certs/ca-certificates.crt",
                "/etc/pki/tls/certs/ca-bundle.crt",
                "/etc/ssl/ca-bundle.pem",
                "/etc/ssl/cert.pem"
            ]
        elif system == "windows":
            return None
        else:
            ca_paths = [
                "/etc/ssl/certs/ca-certificates.crt",
                "/etc/ssl/cert.pem"
            ]
        
        for path in ca_paths:
            if os.path.exists(path):
                return path
        
        return None
    
    def __init__(self):
        self.bridge = None
        self.test_data_dir = "./test_data_simple"
        timestamp = int(time.time() * 1000)
        self.collection_name = f"simple_test_users_{timestamp}"
    
    def initialize(self) -> bool:
        """初始化测试环境"""
        print("🚀 初始化MongoDB缓存简化测试环境...")
        
        try:
            # 创建测试数据目录
            os.makedirs(self.test_data_dir, exist_ok=True)
            
            # 创建数据库队列桥接器
            self.bridge = create_db_queue_bridge()
            
            # 添加带缓存的MongoDB数据库
            self._add_cached_mongodb_database()
            
            # 添加不带缓存的MongoDB数据库
            self._add_non_cached_mongodb_database()
            
            # 触发适配器创建以显示适配器类型
            print("\n🔍 检查适配器类型...")
            self._trigger_adapter_creation()
            
            print("✅ 测试环境初始化完成")
            return True
            
        except Exception as e:
            print(f"❌ 测试环境初始化失败: {e}")
            return False
    
    def _create_cached_config(self) -> PyCacheConfig:
        """创建带缓存的配置"""
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # L1缓存配置
        l1_config = PyL1CacheConfig(5000)
        l1_config.max_memory_mb = 500
        l1_config.enable_stats = False
        cache_config.l1_config = l1_config
        
        # L2缓存配置
        l2_config = PyL2CacheConfig(f"{self.test_data_dir}/mongodb_cache_simple")
        l2_config.max_disk_mb = 2000
        l2_config.compression_level = 1
        l2_config.enable_wal = False
        l2_config.clear_on_startup = False  # 启动时不清空缓存目录
        cache_config.l2_config = l2_config
        
        # TTL配置
        ttl_config = PyTtlConfig(1800)  # 30分钟TTL
        ttl_config.max_ttl_secs = 7200   # 2小时最大TTL
        ttl_config.check_interval_secs = 300  # 5分钟检查间隔
        cache_config.ttl_config = ttl_config
        
        # 压缩配置
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = False  # 禁用压缩以减少CPU开销
        compression_config.threshold_bytes = 1024
        cache_config.compression_config = compression_config
        
        print("  📊 缓存配置: L1(5000条/500MB) + L2(2GB) + TTL(30分钟)")
        return cache_config
    
    def _add_cached_mongodb_database(self):
        """添加带缓存的MongoDB数据库"""
        cache_config = self._create_cached_config()
        
        # TLS配置
        tls_config = PyTlsConfig()
        tls_config.enable()
        
        ca_cert_path = self.get_ca_cert_path()
        if ca_cert_path:
            tls_config.ca_cert_path = ca_cert_path
            print(f"  🔒 使用CA证书路径: {ca_cert_path}")
        else:
            print("  🔒 使用系统默认CA证书存储")
            
        tls_config.client_cert_path = ""
        tls_config.client_key_path = ""
        
        # ZSTD压缩配置
        zstd_config = PyZstdConfig()
        zstd_config.enable()
        zstd_config.compression_level = 3
        zstd_config.compression_threshold = 1024
        
        response = self.bridge.add_mongodb_database(
            alias="mongodb_cached",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=10,
            min_connections=2,
            connection_timeout=5,
            idle_timeout=60,
            max_lifetime=300,
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        
        result = json.loads(response)
        if not result.get("success"):
            raise Exception(f"添加缓存MongoDB数据库失败: {result.get('error')}")
    
    def _add_non_cached_mongodb_database(self):
        """添加不带缓存的MongoDB数据库"""
        # 真正的无缓存配置：不创建任何缓存管理器
        cache_config = None
        
        # TLS配置
        tls_config = PyTlsConfig()
        tls_config.enable()
        
        ca_cert_path = self.get_ca_cert_path()
        if ca_cert_path:
            tls_config.ca_cert_path = ca_cert_path
            print(f"  🔒 非缓存数据库使用CA证书路径: {ca_cert_path}")
        else:
            print("  🔒 非缓存数据库使用系统默认CA证书存储")
            
        tls_config.client_cert_path = ""
        tls_config.client_key_path = ""
        
        # ZSTD压缩配置（禁用）
        zstd_config = PyZstdConfig()
        zstd_config.disable()
        
        response = self.bridge.add_mongodb_database(
            alias="mongodb_non_cached",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=10,
            min_connections=2,
            connection_timeout=5,
            idle_timeout=60,
            max_lifetime=300,
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        
        result = json.loads(response)
        if not result.get("success"):
            raise Exception(f"添加非缓存MongoDB数据库失败: {result.get('error')}")
    
    def _trigger_adapter_creation(self):
        """触发适配器创建以显示适配器类型"""
        try:
            # 对缓存数据库执行一次简单查询
            print("  🔍 触发缓存数据库适配器创建...")
            response = self.bridge.find_one("dummy_collection", "{}", "mongodb_cached")
            # 忽略查询结果，只是为了触发适配器创建
            
            # 对非缓存数据库执行一次简单查询
            print("  🔍 触发非缓存数据库适配器创建...")
            response = self.bridge.find_one("dummy_collection", "{}", "mongodb_non_cached")
            # 忽略查询结果，只是为了触发适配器创建
            
        except Exception as e:
            # 忽略查询错误，因为我们只是想触发适配器创建
            pass
    
    def setup_test_data(self) -> bool:
        """设置测试数据"""
        print("\n🔧 设置MongoDB测试数据...")
        
        try:
            # 创建测试用户数据
            cached_users = [
                TestUser.new(f"cached_user_{i:03d}", f"缓存用户{i}", f"cached_user{i}@example.com", 20 + (i % 50))
                for i in range(1, 51)  # 减少到50条数据
            ]
            
            non_cached_users = [
                TestUser.new(f"non_cached_user_{i:03d}", f"非缓存用户{i}", f"non_cached_user{i}@example.com", 20 + (i % 50))
                for i in range(1, 51)  # 减少到50条数据
            ]
            
            # 创建测试数据到缓存数据库
            for i, user in enumerate(cached_users):
                response = self.bridge.create(self.collection_name, user.to_json(), "mongodb_cached")
                result = json.loads(response)
                if not result.get("success"):
                    raise Exception(result.get('error'))
                if i == 0:
                    print(f"  ✅ 创建缓存用户数据成功")
            
            # 创建测试数据到非缓存数据库
            for i, user in enumerate(non_cached_users):
                response = self.bridge.create(self.collection_name, user.to_json(), "mongodb_non_cached")
                result = json.loads(response)
                if not result.get("success"):
                    raise Exception(result.get('error'))
                if i == 0:
                    print(f"  ✅ 创建非缓存用户数据成功")
            
            print(f"  ✅ 创建了 {len(cached_users) + len(non_cached_users)} 条测试记录（每个数据库{len(cached_users)}条）")
            print(f"  📝 使用集合名称: {self.collection_name}")
            return True
            
        except Exception as e:
            print(f"❌ 设置测试数据失败: {e}")
            return False
    
    def warmup_cache(self) -> bool:
        """缓存预热"""
        print("\n🔥 缓存预热...")
        
        try:
            # 预热查询
            query_conditions = json.dumps([
                {"field": "age", "operator": "Gte", "value": 25},
                {"field": "age", "operator": "Lte", "value": 35},
                {"field": "name", "operator": "Contains", "value": "用户"},
                {"field": "email", "operator": "Contains", "value": "@example.com"}
            ])
            self.bridge.find(self.collection_name, query_conditions, "mongodb_cached")
            
            # 按ID查询预热
            for i in range(1, 11):
                self.bridge.find_by_id(self.collection_name, f"cached_user_{i:03d}", "mongodb_cached")
            
            print("  ✅ 缓存预热完成")
            return True
            
        except Exception as e:
            print(f"❌ 缓存预热失败: {e}")
            return False
    
    def test_500_queries(self) -> tuple[bool, float, float]:
        """测试500次查询性能对比"""
        print("\n🔍 测试500次查询性能对比...")
        
        try:
            # 构建查询条件
            query_conditions = json.dumps([
                {"field": "age", "operator": "Gte", "value": 25},
                {"field": "age", "operator": "Lte", "value": 35},
                {"field": "name", "operator": "Contains", "value": "用户"},
                {"field": "email", "operator": "Contains", "value": "@example.com"}
            ])
            
            # 测试缓存数据库查询（500次）
            print("  🔄 执行缓存查询...")
            start_time = time.time()
            for i in range(500):
                self.bridge.find(self.collection_name, query_conditions, "mongodb_cached")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库查询（500次）
            print("  🔄 执行非缓存查询...")
            start_time = time.time()
            for i in range(500):
                self.bridge.find(self.collection_name, query_conditions, "mongodb_non_cached")
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            print(f"  ✅ 缓存查询总耗时: {cached_duration:.2f}ms")
            print(f"  ✅ 非缓存查询总耗时: {non_cached_duration:.2f}ms")
            print(f"  ✅ 平均单次查询（缓存）: {cached_duration/500:.2f}ms")
            print(f"  ✅ 平均单次查询（非缓存）: {non_cached_duration/500:.2f}ms")
            
            if non_cached_duration > 0:
                improvement_ratio = non_cached_duration / cached_duration
                print(f"  📈 性能提升: {improvement_ratio:.2f}x")
            
            return True, cached_duration, non_cached_duration
            
        except Exception as e:
            print(f"❌ 500次查询测试失败: {e}")
            return False, 0.0, 0.0
    
    def cleanup(self):
        """清理资源"""
        try:
            if os.path.exists(self.test_data_dir):
                import shutil
                shutil.rmtree(self.test_data_dir)
                print(f"  🧹 清理测试数据目录: {self.test_data_dir}")
        except Exception as e:
            print(f"⚠️ 清理资源时出错: {e}")
    
    def run_test(self) -> int:
        """运行完整测试"""
        try:
            # 初始化测试环境
            if not self.initialize():
                return 1
            
            # 设置测试数据
            if not self.setup_test_data():
                return 1
            
            # 缓存预热
            if not self.warmup_cache():
                return 1
            
            # 执行500次查询测试
            success, cached_time, non_cached_time = self.test_500_queries()
            if not success:
                return 1
            
            # 验证缓存禁用功能
            print("\n🔍 验证缓存禁用功能...")
            if non_cached_time < 200:
                print(f"❌ 缓存禁用失败！非缓存查询时间 {non_cached_time:.2f}ms < 200ms")
                print("   这表明缓存可能未被正确禁用")
                return 2  # 返回非0错误码
            else:
                print(f"✅ 缓存禁用成功！非缓存查询时间 {non_cached_time:.2f}ms > 200ms")
                print("   缓存功能正常工作")
            
            print("\n🎯 测试完成！")
            return 0
            
        except Exception as e:
            print(f"❌ 测试执行失败: {e}")
            return 1
        finally:
            self.cleanup()


def display_version_info():
    """显示版本信息"""
    try:
        print(f"📦 RatQuickDB 版本: {get_version()}")
        print(f"📋 库信息: {get_info()}")
        print(f"🏷️  库名称: {get_name()}")
    except Exception as e:
        print(f"⚠️ 无法获取版本信息: {e}")


def main() -> int:
    """主函数"""
    print("🚀 RatQuickDB Python MongoDB 缓存简化测试")
    print("===========================================")
    
    # 显示版本信息
    display_version_info()
    print()
    
    # 创建并运行测试
    test = MongoDbCacheSimpleTest()
    
    try:
        return test.run_test()
    except KeyboardInterrupt:
        print("\n🛑 收到键盘中断，退出测试")
        test.cleanup()
        return 0
    except Exception as e:
        print(f"⚠️ 程序执行出错: {e}")
        test.cleanup()
        return 1


if __name__ == "__main__":
    exit(main())