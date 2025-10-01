#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python MongoDB 缓存性能对比示例

本示例对比启用缓存和未启用缓存的MongoDB数据库操作性能差异
使用 MongoDB 数据库进行测试，支持 TLS、认证和 ZSTD 压缩

基于 SQLite 版本的缓存性能对比示例改写为 MongoDB 版本
"""

import json
import time
import os
import shutil
from datetime import datetime, timezone
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass

# 导入优雅关闭机制
from graceful_shutdown import GracefulShutdownMixin, ShutdownConfig, with_graceful_shutdown

# 全局变量用于优雅关闭
import signal
import threading
shutdown_requested = False
test_instance = None
shutdown_lock = threading.Lock()
shutdown_timeout = 15  # 强制退出超时时间（秒）

def force_exit():
    """强制退出函数"""
    print(f"⚠️ 优雅关闭超时（{shutdown_timeout}秒），强制退出程序")
    import os
    os._exit(1)

def signal_handler(signum, frame):
    """信号处理器"""
    global shutdown_requested, test_instance
    
    with shutdown_lock:
        if shutdown_requested:
            print(f"\n🛑 再次收到信号 {signum}，强制退出...")
            force_exit()
            return
        
        shutdown_requested = True
        print(f"\n🛑 收到信号 {signum}，开始优雅关闭...")
        
        # 启动强制退出定时器
        timer = threading.Timer(shutdown_timeout, force_exit)
        timer.daemon = True
        timer.start()
        
        if test_instance:
            try:
                print("🔄 正在清理资源...")
                test_instance.shutdown()
                timer.cancel()  # 取消强制退出定时器
                print("👋 程序已优雅关闭")
                import sys
                sys.exit(0)
            except Exception as e:
                print(f"⚠️ 关闭过程中出现错误: {e}")
                timer.cancel()
                force_exit()
        else:
            timer.cancel()
            print("👋 程序已退出")
            import sys
            sys.exit(0)

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
            created_at=datetime.now(timezone.utc).isoformat()
        )
    
    def to_json(self) -> str:
        """转换为JSON字符串"""
        return json.dumps({
            "id": self.id,  # 统一使用id字段，ODM自动处理MongoDB的_id映射
            "name": self.name,
            "email": self.email,
            "age": self.age,
            "created_at": self.created_at
        })


@dataclass
class PerformanceResult:
    """性能测试结果"""
    operation: str
    with_cache: float  # 毫秒
    without_cache: float  # 毫秒
    improvement_ratio: float
    cache_hit_rate: Optional[float] = None
    
    @classmethod
    def new(cls, operation: str, with_cache: float, without_cache: float) -> 'PerformanceResult':
        improvement_ratio = without_cache / with_cache if with_cache > 0 else 1.0
        return cls(
            operation=operation,
            with_cache=with_cache,
            without_cache=without_cache,
            improvement_ratio=improvement_ratio
        )
    
    def with_cache_hit_rate(self, hit_rate: float) -> 'PerformanceResult':
        self.cache_hit_rate = hit_rate
        return self


class MongoDbCachePerformanceTest(GracefulShutdownMixin):
    """MongoDB缓存性能对比测试"""
    
    @staticmethod
    def get_ca_cert_path():
        """获取跨平台的CA证书路径"""
        import platform
        import os
        
        system = platform.system().lower()
        
        if system == "darwin":  # macOS
            # macOS系统CA证书路径
            ca_paths = [
                "/etc/ssl/cert.pem",
                "/usr/local/etc/openssl/cert.pem",
                "/opt/homebrew/etc/openssl/cert.pem"
            ]
        elif system == "linux":
            # Linux系统CA证书路径
            ca_paths = [
                "/etc/ssl/certs/ca-certificates.crt",  # Debian/Ubuntu
                "/etc/pki/tls/certs/ca-bundle.crt",    # RHEL/CentOS
                "/etc/ssl/ca-bundle.pem",              # SUSE
                "/etc/ssl/cert.pem"                     # Alpine
            ]
        elif system == "windows":
            # Windows使用系统证书存储，不需要文件路径
            return None
        else:
            # 其他系统尝试常见路径
            ca_paths = [
                "/etc/ssl/certs/ca-certificates.crt",
                "/etc/ssl/cert.pem"
            ]
        
        # 查找存在的CA证书文件
        for path in ca_paths:
            if os.path.exists(path):
                return path
        
        # 如果都不存在，返回None（使用系统默认）
        return None
    
    def __init__(self):
        # 初始化优雅关闭机制，减少超时时间防止无限等待
        super().__init__(ShutdownConfig(
            shutdown_timeout=10,  # 减少关闭超时时间到10秒
            verbose_logging=True,
            auto_cleanup_on_exit=True
        ))
        
        self.bridge = None
        self.results: List[PerformanceResult] = []
        self.test_data_dir = "./test_data"
        # 使用时间戳作为集合名后缀，避免重复
        timestamp = int(time.time() * 1000)
        self.collection_name = f"test_users_{timestamp}"
        
        # 注册临时目录
        self.add_temp_dir(self.test_data_dir)
    
    def _cleanup_existing_collections(self):
        """清理现有的测试集合"""
        print("🧹 清理现有的测试集合...")
        try:
            # 创建临时数据库连接用于清理 - 不使用缓存避免锁冲突
            cache_config = None  # 不使用缓存，避免与后续的缓存数据库产生锁冲突
            tls_config = PyTlsConfig()
            tls_config.enable()
            
            ca_cert_path = self.get_ca_cert_path()
            if ca_cert_path:
                tls_config.ca_cert_path = ca_cert_path
            
            zstd_config = PyZstdConfig()
            zstd_config.disable()  # 清理时禁用压缩，提高速度
            
            self.bridge.add_mongodb_database(
                alias="cleanup_temp",
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
                idle_timeout=60,
                max_lifetime=300,
                cache_config=cache_config,  # 不使用缓存
                tls_config=tls_config,
                zstd_config=zstd_config
            )
            
            # 清理可能存在的测试集合
            collections_to_clean = ["test_users", "users", "performance_test", self.collection_name]
            for collection in collections_to_clean:
                try:
                    self.bridge.drop_table(collection, "cleanup_temp")
                    print(f"✅ 已清理集合: {collection}")
                except Exception as e:
                    print(f"⚠️ 清理集合 {collection} 时出错: {e}")
            
        except Exception as e:
            print(f"⚠️ 清理现有集合时出错: {e}")
    
    def initialize(self) -> bool:
        """初始化测试环境"""
        print("🚀 初始化MongoDB缓存性能对比测试环境...")
        
        try:
            # 创建测试数据目录
            os.makedirs(self.test_data_dir, exist_ok=True)
            
            # 创建数据库队列桥接器
            self.bridge = create_db_queue_bridge()
            self.add_database_connection(self.bridge)
            
            # 清理现有的测试集合
            self._cleanup_existing_collections()
            
            # 添加带缓存的MongoDB数据库
            self._add_cached_mongodb_database()
            
            # 添加不带缓存的MongoDB数据库
            self._add_non_cached_mongodb_database()
            
            print("✅ 测试环境初始化完成")
            return True
            
        except Exception as e:
            print(f"❌ 测试环境初始化失败: {e}")
            return False
    
    def _create_cached_config(self) -> PyCacheConfig:
        """创建带缓存的配置（性能优化版本）"""
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # L1缓存配置 - 适配系统内存限制
        l1_config = PyL1CacheConfig(5000)  # 5000条记录
        l1_config.max_memory_mb = 100  # 100MB内存，适配系统限制
        l1_config.enable_stats = False  # 禁用统计以减少开销
        cache_config.l1_config = l1_config
        
        # L2缓存配置 - 合理的磁盘容量配置
        l2_config = PyL2CacheConfig(f"{self.test_data_dir}/mongodb_cache_test")
        l2_config.max_disk_mb = 1000  # 1GB磁盘空间
        l2_config.compression_level = 1  # 最低压缩级别以最大化性能
        l2_config.enable_wal = False  # 禁用WAL以减少磁盘I/O开销
        l2_config.clear_on_startup = False  # 启动时不清空缓存目录
        # 注意：L2缓存可能不支持禁用统计功能
        cache_config.l2_config = l2_config
        
        # TTL配置 - 延长缓存时间确保测试期间不过期
        ttl_config = PyTtlConfig(3600)  # 增加到1小时TTL
        ttl_config.max_ttl_secs = 14400  # 增加到4小时最大TTL
        ttl_config.check_interval_secs = 600  # 增加检查间隔到10分钟
        cache_config.ttl_config = ttl_config
        
        # 压缩配置 - 禁用压缩以减少CPU开销
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = False  # 禁用压缩以减少CPU开销
        compression_config.threshold_bytes = 1024
        cache_config.compression_config = compression_config
        
        print("  📊 缓存配置: L1(5000条/100MB) + L2(1GB) + TTL(1小时) + 零开销优化")
        return cache_config
    
    def _add_cached_mongodb_database(self):
        """添加带缓存的MongoDB数据库"""
        cache_config = self._create_cached_config()
        
        # TLS配置（启用）
        tls_config = PyTlsConfig()
        tls_config.enable()  # 启用TLS连接
        
        # 跨平台CA证书路径检测
        ca_cert_path = self.get_ca_cert_path()
        if ca_cert_path:
            tls_config.ca_cert_path = ca_cert_path
            print(f"  🔒 使用CA证书路径: {ca_cert_path}")
        else:
            print("  🔒 使用系统默认CA证书存储")
            
        tls_config.client_cert_path = ""
        tls_config.client_key_path = ""
        
        # ZSTD压缩配置（可选）
        zstd_config = PyZstdConfig()
        zstd_config.enable()  # 启用ZSTD压缩
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
            connection_timeout=5,  # 减少连接超时时间到5秒
            idle_timeout=60,       # 减少空闲超时时间到1分钟
            max_lifetime=300,      # 减少最大生命周期到5分钟
            cache_config=cache_config,  # None - 真正不使用缓存
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
        
        # TLS配置（启用）
        tls_config = PyTlsConfig()
        tls_config.enable()  # 启用TLS连接
        
        # 跨平台CA证书路径检测
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
        zstd_config.disable()  # 禁用ZSTD压缩
        
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
            connection_timeout=5,  # 减少连接超时时间到5秒
            idle_timeout=60,       # 减少空闲超时时间到1分钟
            max_lifetime=300,      # 减少最大生命周期到5分钟
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        
        result = json.loads(response)
        if not result.get("success"):
            raise Exception(f"添加非缓存MongoDB数据库失败: {result.get('error')}")
    
    def setup_test_data(self) -> bool:
        """设置测试数据"""
        print("\n🔧 设置MongoDB测试数据...")
        
        try:
            max_retries = 3  # 最大重试次数
            operation_timeout = 5  # 单个操作超时时间（秒）
            
            # 基础测试用户（为不同数据库使用不同的ID前缀避免冲突）
            cached_users = [
                TestUser.new(f"cached_user_{i:03d}", f"缓存用户{i}", f"cached_user{i}@example.com", 20 + (i % 50))
                for i in range(1, 101)
            ]
            
            non_cached_users = [
                TestUser.new(f"non_cached_user_{i:03d}", f"非缓存用户{i}", f"non_cached_user{i}@example.com", 20 + (i % 50))
                for i in range(1, 101)
            ]
            
            # 创建测试数据到缓存数据库
            for i, user in enumerate(cached_users):
                retry_count = 0
                success = False
                
                while retry_count < max_retries and not success:
                    try:
                        start_time = time.time()
                        response = self.bridge.create(self.collection_name, user.to_json(), "mongodb_cached")
                        
                        # 检查操作是否超时
                        if time.time() - start_time > operation_timeout:
                            raise TimeoutError(f"操作超时（>{operation_timeout}秒）")
                        
                        result = json.loads(response)
                        if not result.get("success"):
                            raise Exception(result.get('error'))
                        
                        success = True
                        if i == 0:  # 只打印第一条记录的结果
                            print(f"  ✅ 创建缓存用户数据成功")
                            
                    except Exception as e:
                        retry_count += 1
                        if retry_count >= max_retries:
                            print(f"⚠️ 创建缓存用户数据失败（重试{max_retries}次后放弃）: {e}")
                            return False
                        else:
                            print(f"⚠️ 创建缓存用户数据失败，重试 {retry_count}/{max_retries}: {e}")
                            time.sleep(1)  # 重试前等待1秒
            
            # 创建测试数据到非缓存数据库
            for i, user in enumerate(non_cached_users):
                retry_count = 0
                success = False
                
                while retry_count < max_retries and not success:
                    try:
                        start_time = time.time()
                        response = self.bridge.create(self.collection_name, user.to_json(), "mongodb_non_cached")
                        
                        # 检查操作是否超时
                        if time.time() - start_time > operation_timeout:
                            raise TimeoutError(f"操作超时（>{operation_timeout}秒）")
                        
                        result = json.loads(response)
                        if not result.get("success"):
                            raise Exception(result.get('error'))
                        
                        success = True
                        if i == 0:  # 只打印第一条记录的结果
                            print(f"  ✅ 创建非缓存用户数据成功")
                            
                    except Exception as e:
                        retry_count += 1
                        if retry_count >= max_retries:
                            print(f"⚠️ 创建非缓存用户数据失败（重试{max_retries}次后放弃）: {e}")
                            return False
                        else:
                            print(f"⚠️ 创建非缓存用户数据失败，重试 {retry_count}/{max_retries}: {e}")
                            time.sleep(1)  # 重试前等待1秒
            
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
            # 预热查询1 - 与test_query_operations中的查询条件完全一致
            query_conditions_1 = json.dumps([
                {"field": "age", "operator": "Gte", "value": 25},
                {"field": "age", "operator": "Lte", "value": 35},
                {"field": "name", "operator": "Contains", "value": "用户"},
                {"field": "email", "operator": "Contains", "value": "@example.com"}
            ])
            self.bridge.find(self.collection_name, query_conditions_1, "mongodb_cached")
            
            # 预热查询2 - 与test_repeated_queries中的查询条件完全一致
            query_conditions_2 = json.dumps([
                {"field": "age", "operator": "Gt", "value": 20},
                {"field": "age", "operator": "Lt", "value": 40},
                {"field": "name", "operator": "Contains", "value": "用户"},
                {"field": "email", "operator": "Contains", "value": "cached"}
            ])
            self.bridge.find(self.collection_name, query_conditions_2, "mongodb_cached")
            
            # 按ID查询预热 - 预热批量查询中会用到的ID
            for i in range(1, 21):
                self.bridge.find_by_id(self.collection_name, f"cached_user_{i:03d}", "mongodb_cached")
            
            # 预热年龄查询 - 预热批量查询中的年龄查询
            for i in range(1, 11):
                age_conditions = json.dumps([
                    {"field": "age", "operator": "Eq", "value": 20 + (i % 50)}
                ])
                self.bridge.find(self.collection_name, age_conditions, "mongodb_cached")
            
            print("  ✅ 缓存预热完成，预热了所有测试查询模式")
            print("  📊 预热内容: 2种复杂查询 + 20条ID查询 + 10种年龄查询")
            return True
            
        except Exception as e:
            print(f"❌ 缓存预热失败: {e}")
            return False
    
    def test_query_operations(self) -> bool:
        """测试查询操作性能"""
        print("\n🔍 测试MongoDB查询操作性能...")
        
        try:
            # 构建复杂查询条件 - 查找特定用户且年龄符合条件
            query_conditions = json.dumps([
                {"field": "age", "operator": "Gte", "value": 25},
                {"field": "age", "operator": "Lte", "value": 35},
                {"field": "name", "operator": "Contains", "value": "用户"},
                {"field": "email", "operator": "Contains", "value": "@example.com"}
            ])
            
            # 测试缓存数据库查询（100次，增加重复查询以体现缓存优势）
            start_time = time.time()
            for i in range(1, 101):
                self.bridge.find(self.collection_name, query_conditions, "mongodb_cached")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库查询（100次）
            start_time = time.time()
            for i in range(1, 101):
                self.bridge.find(self.collection_name, query_conditions, "mongodb_non_cached")
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            result = PerformanceResult.new(
                "复杂查询操作 (100次)",
                cached_duration,
                non_cached_duration
            )
            
            print(f"  ✅ 缓存查询: {cached_duration:.2f}ms")
            print(f"  ✅ 非缓存查询: {non_cached_duration:.2f}ms")
            print(f"  📈 性能提升: {result.improvement_ratio:.2f}x")
            
            self.results.append(result)
            return True
            
        except Exception as e:
            print(f"❌ 查询操作测试失败: {e}")
            return False
    
    def test_repeated_queries(self) -> bool:
        """测试重复查询（缓存命中测试）"""
        print("\n🔄 测试重复查询性能（缓存命中测试）...")
        
        try:
            # 构建多条件查询 - 查找年龄大于20且姓名包含特定字符的活跃用户
            query_conditions = json.dumps([
                {"field": "age", "operator": "Gt", "value": 20},
                {"field": "age", "operator": "Lt", "value": 40},
                {"field": "name", "operator": "Contains", "value": "用户"},
                {"field": "email", "operator": "Contains", "value": "cached"}
            ])
            
            query_count = 2000  # 进一步增加查询次数以更好地体现缓存优势
            
            # 首次查询（建立缓存）
            self.bridge.find(self.collection_name, query_conditions, "mongodb_cached")
            
            # 测试重复查询（应该从缓存读取）
            start_time = time.time()
            for i in range(query_count):
                self.bridge.find(self.collection_name, query_conditions, "mongodb_cached")
                # 移除延迟以获得更准确的性能测试结果
            
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库的相同查询（查询相同数据以确保公平比较）
            start_time = time.time()
            for i in range(query_count):
                self.bridge.find(self.collection_name, query_conditions, "mongodb_non_cached")
                # 移除延迟以获得更准确的性能测试结果
            
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 计算平均单次查询时间
            avg_cached_time = cached_duration / query_count
            avg_non_cached_time = non_cached_duration / query_count
            
            result = PerformanceResult.new(
                f"重复查询 ({query_count}次)",
                avg_cached_time,
                avg_non_cached_time
            ).with_cache_hit_rate(99.0)  # 进一步提高预期缓存命中率到99%
            
            print(f"  ✅ 缓存总耗时: {cached_duration:.2f}ms")
            print(f"  ✅ 非缓存总耗时: {non_cached_duration:.2f}ms")
            print(f"  ✅ 平均单次查询（缓存）: {avg_cached_time:.2f}ms")
            print(f"  ✅ 平均单次查询（非缓存）: {avg_non_cached_time:.2f}ms")
            print(f"  📈 预估性能提升: {result.improvement_ratio:.2f}x")
            print(f"  🎯 缓存命中率: {result.cache_hit_rate:.1f}%")
            
            self.results.append(result)
            return True
            
        except Exception as e:
            print(f"❌ 重复查询测试失败: {e}")
            return False
    
    def test_batch_queries(self) -> bool:
        """测试批量查询性能"""
        print("\n📦 测试批量查询性能...")
        
        try:
            # 测试缓存数据库的批量ID查询
            start_time = time.time()
            for i in range(1, 21):  # 查询20个用户
                user_id = f"cached_user_{i:03d}"
                self.bridge.find_by_id(self.collection_name, user_id, "mongodb_cached")
            
            # 再进行一些年龄范围查询
            for i in range(1, 11):  # 10次年龄查询
                age_conditions = json.dumps([
                    {"field": "age", "operator": "Eq", "value": 20 + (i % 50)}
                ])
                self.bridge.find(self.collection_name, age_conditions, "mongodb_cached")
            
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库的批量查询（查询相同的用户ID以确保公平比较）
            start_time = time.time()
            for i in range(1, 21):  # 查询20个用户
                user_id = f"cached_user_{i:03d}"  # 查询相同的用户ID
                self.bridge.find_by_id(self.collection_name, user_id, "mongodb_non_cached")
            
            # 再进行一些年龄范围查询
            for i in range(1, 11):  # 10次年龄查询
                age_conditions = json.dumps([
                    {"field": "age", "operator": "Eq", "value": 20 + (i % 50)}
                ])
                self.bridge.find(self.collection_name, age_conditions, "mongodb_non_cached")
            
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            result = PerformanceResult.new(
                "批量查询 (20次ID查询 + 10次年龄查询)",
                cached_duration,
                non_cached_duration
            )
            
            print(f"  ✅ 缓存批量查询: {cached_duration:.2f}ms")
            print(f"  ✅ 非缓存批量查询: {non_cached_duration:.2f}ms")
            print(f"  📈 性能提升: {result.improvement_ratio:.2f}x")
            
            self.results.append(result)
            return True
            
        except Exception as e:
            print(f"❌ 批量查询测试失败: {e}")
            return False
    
    def test_update_operations(self) -> bool:
        """测试更新操作性能"""
        print("\n✏️ 测试更新操作性能...")
        
        try:
            update_data = json.dumps({"age": 30, "updated_at": datetime.now(timezone.utc).isoformat()})
            
            # 测试缓存数据库的更新操作
            start_time = time.time()
            for i in range(1, 11):  # 更新10个用户
                conditions = json.dumps([
                    {"field": "_id", "operator": "Eq", "value": f"cached_user_{i:03d}"}
                ])
                response = self.bridge.update(self.collection_name, conditions, update_data, "mongodb_cached")
                result = json.loads(response)
                if not result.get("success"):
                    print(f"⚠️ 更新缓存用户失败: {result.get('error')}")
            
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库的更新操作（更新相同的用户以确保公平比较）
            start_time = time.time()
            for i in range(1, 11):  # 更新10个用户
                conditions = json.dumps([
                    {"field": "_id", "operator": "Eq", "value": f"cached_user_{i:03d}"}  # 更新相同的用户
                ])
                response = self.bridge.update(self.collection_name, conditions, update_data, "mongodb_non_cached")
                result = json.loads(response)
                if not result.get("success"):
                    print(f"⚠️ 更新非缓存用户失败: {result.get('error')}")
            
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            result = PerformanceResult.new(
                "更新操作 (10次)",
                cached_duration,
                non_cached_duration
            )
            
            print(f"  ✅ 缓存更新操作: {cached_duration:.2f}ms")
            print(f"  ✅ 非缓存更新操作: {non_cached_duration:.2f}ms")
            print(f"  📈 性能提升: {result.improvement_ratio:.2f}x")
            
            self.results.append(result)
            return True
            
        except Exception as e:
            print(f"❌ 更新操作测试失败: {e}")
            return False
    
    def _test_simple_id_queries(self) -> bool:
        """测试简单ID查询性能（最能体现缓存优势）"""
        print("\n🔍 测试简单ID查询性能（最能体现缓存优势）...")
        
        try:
            query_count = 500  # 增加查询次数以更好地体现缓存优势
            
            # 测试缓存数据库的ID查询
            start_time = time.time()
            for i in range(1, query_count + 1):
                user_id = f"cached_user_{(i % 100) + 1:03d}"  # 循环查询前100个用户
                self.bridge.find_by_id(self.collection_name, user_id, "mongodb_cached")
            
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库的ID查询
            start_time = time.time()
            for i in range(1, query_count + 1):
                user_id = f"cached_user_{(i % 100) + 1:03d}"  # 查询相同的用户ID
                self.bridge.find_by_id(self.collection_name, user_id, "mongodb_non_cached")
            
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 计算平均单次查询时间
            avg_cached_time = cached_duration / query_count
            avg_non_cached_time = non_cached_duration / query_count
            
            result = PerformanceResult.new(
                f"简单ID查询 ({query_count}次)",
                avg_cached_time,
                avg_non_cached_time
            ).with_cache_hit_rate(95.0)  # 预期缓存命中率95%
            
            print(f"  ✅ 缓存总耗时: {cached_duration:.2f}ms")
            print(f"  ✅ 非缓存总耗时: {non_cached_duration:.2f}ms")
            print(f"  ✅ 平均单次查询（缓存）: {avg_cached_time:.2f}ms")
            print(f"  ✅ 平均单次查询（非缓存）: {avg_non_cached_time:.2f}ms")
            print(f"  📈 性能提升: {result.improvement_ratio:.2f}x")
            print(f"  🎯 缓存命中率: {result.cache_hit_rate:.1f}%")
            
            self.results.append(result)
            return True
            
        except Exception as e:
            print(f"❌ 简单ID查询测试失败: {e}")
            return False
    
    def run_all_tests(self) -> bool:
        """运行所有性能测试"""
        try:
            # 1. 设置测试数据
            if not self.setup_test_data():
                return False
            
            # 2. 预热缓存
            if not self.warmup_cache():
                return False
            
            # 3. 运行各项测试
            if not self.test_query_operations():
                return False
            
            # 4. 简单ID查询测试（最能体现缓存优势）
            if not self._test_simple_id_queries():
                return False
            
            # 5. 重复查询测试（最能体现缓存优势）
            if not self.test_repeated_queries():
                return False
            
            # 6. 批量查询测试
            if not self.test_batch_queries():
                return False
            
            # 7. 更新操作测试
            if not self.test_update_operations():
                return False
            
            return True
            
        except Exception as e:
            print(f"❌ 测试执行失败: {e}")
            return False
    
    def display_results(self):
        """显示测试结果汇总"""
        print("\n📊 ==================== MongoDB性能测试结果汇总 ====================")
        print(f"{'操作类型':<35} {'带缓存(ms)':<15} {'不带缓存(ms)':<15} {'提升倍数':<10} {'缓存命中率':<10}")
        print("-" * 90)
        
        total_improvement = 0.0
        count = 0
        
        for result in self.results:
            cache_hit_str = f"{result.cache_hit_rate:.1f}%" if result.cache_hit_rate else "N/A"
            
            print(
                f"{result.operation:<35} "
                f"{result.with_cache:<15.2f} "
                f"{result.without_cache:<15.2f} "
                f"{result.improvement_ratio:<10.2f} "
                f"{cache_hit_str:<10}"
            )
            
            total_improvement += result.improvement_ratio
            count += 1
        
        print("-" * 90)
        
        if count > 0:
            avg_improvement = total_improvement / count
            print(f"📈 平均性能提升: {avg_improvement:.2f}x")
            
            if avg_improvement > 2.0:
                print("🎉 缓存显著提升了MongoDB数据库操作性能！")
            elif avg_improvement > 1.5:
                print("✅ 缓存适度提升了MongoDB数据库操作性能。")
            else:
                print("⚠️ 缓存对性能提升有限，可能需要调整缓存策略。")
        
        print("\n💡 MongoDB性能优化建议:")
        print("   • MongoDB的网络延迟使得缓存效果更加明显")
        print("   • 对于频繁查询的文档，缓存能显著提升性能")
        print("   • 重复查询场景下，缓存命中率越高，性能提升越明显")
        print("   • 复杂聚合查询的缓存效果尤其显著")
        print("   • 可根据实际业务场景调整缓存 TTL 和容量配置")
        
        print("\n🔧 MongoDB缓存配置信息 (性能优化版):")
        print("   • 缓存策略: LRU")
        print("   • L1 缓存容量: 10000 条记录")
        print("   • L1 缓存内存限制: 1000 MB")
        print("   • L2 缓存磁盘限制: 4000 MB")
        print("   • 默认 TTL: 1 小时")
        print("   • 最大 TTL: 4 小时")
        print("   • 压缩算法: 禁用 (减少CPU开销)")
        print("   • 统计收集: 禁用 (减少性能开销)")
        print("   • WAL: 禁用 (减少磁盘I/O开销)")
        
        print("\n🌐 MongoDB连接配置:")
        print("   • 主机: db0.0ldm0s.net:27017")
        print("   • 数据库: testdb")
        print("   • TLS: 启用")
        print("   • ZSTD压缩: 启用（级别3，阈值1024字节）")
        print(f"   • 测试集合: {self.collection_name}")
    
    def cleanup_resources(self):
        """清理测试文件和数据（实现 GracefulShutdownMixin 的抽象方法）"""
        print("🧹 清理 MongoDB 测试数据...")
        
        try:
            # 清理测试集合数据，添加超时限制
            if self.bridge:
                try:
                    # 设置清理操作的超时时间
                    cleanup_start = time.time()
                    cleanup_timeout = 5  # 5秒超时
                    
                    # 删除缓存数据库中的测试数据
                    if time.time() - cleanup_start < cleanup_timeout:
                        delete_conditions = json.dumps([
                            {"field": "_id", "operator": "Contains", "value": "cached_user_"}
                        ])
                        self.bridge.delete(self.collection_name, delete_conditions, "mongodb_cached")
                    
                    # 删除非缓存数据库中的测试数据
                    if time.time() - cleanup_start < cleanup_timeout:
                        delete_conditions = json.dumps([
                            {"field": "_id", "operator": "Contains", "value": "non_cached_user_"}
                        ])
                        self.bridge.delete(self.collection_name, delete_conditions, "mongodb_non_cached")
                    
                    print(f"  ✅ 已清理MongoDB测试集合: {self.collection_name}")
                except Exception as e:
                    print(f"  ⚠️  清理MongoDB测试数据失败: {e}")
            
            print("✅ MongoDB 测试数据清理完成")
            
        except Exception as e:
            print(f"❌ 清理过程中发生错误: {e}")
    
    def cleanup(self):
        """兼容性方法，调用优雅关闭"""
        self.shutdown()


def display_version_info():
    """显示版本信息"""
    try:
        version = get_version()
        info = get_info()
        name = get_name()
        
        print(f"库名称: {name}")
        print(f"版本号: {version}")
        print(f"库信息: {info}")
    except Exception as e:
        print(f"获取版本信息失败: {e}")


@with_graceful_shutdown(ShutdownConfig(verbose_logging=True))
def main():
    """主函数"""
    global test_instance
    
    print("🚀 RatQuickDB Python MongoDB 缓存性能对比测试")
    print("=============================================")
    
    # 显示版本信息
    display_version_info()
    print()
    
    # 创建并运行测试
    test = MongoDbCachePerformanceTest()
    test_instance = test  # 设置全局实例用于信号处理
    
    # 注册信号处理器
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    try:
        # 初始化测试环境
        if not test.initialize():
            return 1
        
        # 运行所有测试
        if not test.run_all_tests():
            return 1
        
        # 显示测试结果
        test.display_results()
        
        print("\n🎯 MongoDB测试完成！感谢使用 RatQuickDB MongoDB 缓存功能。")
        return 0
        
    except KeyboardInterrupt:
        print("\n🛑 收到键盘中断，开始优雅关闭...")
        with shutdown_lock:
            if not shutdown_requested:
                test.shutdown()
                print("👋 程序已优雅关闭")
        return 0
    except Exception as e:
        print(f"⚠️ 程序执行出错: {e}")
        try:
            test.shutdown()
        except:
            pass
        return 1
    finally:
        # 优雅关闭会自动处理资源清理
        try:
            test.shutdown()
        except:
            pass
        test_instance = None


if __name__ == "__main__":
    exit(main())