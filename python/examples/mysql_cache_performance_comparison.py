#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python MySQL 缓存性能对比示例

本示例对比启用缓存和未启用缓存的MySQL数据库操作性能差异
使用 MySQL 数据库进行测试

基于 MongoDB 版本的缓存性能对比示例改写为 MySQL 版本
"""

import json
import time
import os
import shutil
from datetime import datetime
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
    )
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    exit(1)


@dataclass
class TestUser:
    """测试用户数据结构"""
    id: int
    name: str
    email: str
    age: int
    city: str
    created_at: str
    
    @classmethod
    def new(cls, user_id: int, name: str, email: str, age: int, city: str) -> 'TestUser':
        return cls(
            id=user_id,
            name=name,
            email=email,
            age=age,
            city=city,
            created_at=datetime.utcnow().isoformat() + "Z"
        )
    
    def to_json(self) -> str:
        """转换为JSON字符串（不包含id，让MySQL自动生成）"""
        return json.dumps({
            "name": self.name,
            "email": self.email,
            "age": self.age,
            "city": self.city,
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


class MySqlCachePerformanceTest(GracefulShutdownMixin):
    """MySQL缓存性能对比测试"""
    
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
        # 使用时间戳作为表名后缀，避免重复
        timestamp = int(time.time() * 1000)
        self.table_name = f"test_users_{timestamp}"
        
        # 注册临时目录
        self.add_temp_dir(self.test_data_dir)
    
    def _cleanup_existing_tables(self):
        """清理现有的测试表"""
        print("🧹 清理现有的测试表...")
        try:
            # 创建临时桥接器进行清理
            temp_bridge = create_db_queue_bridge()
            
            # 添加MySQL数据库连接
            result = temp_bridge.add_mysql_database(
                alias="mysql_cleanup",
                host="172.16.0.21",
                port=3306,
                database="testdb",
                username="testdb",
                password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
                max_connections=5,
                min_connections=1,
                connection_timeout=10,
                idle_timeout=300,
                max_lifetime=600
            )
            
            result_data = json.loads(result)
            if result_data.get("success"):
                # 删除测试表中的数据
                tables_to_clean = ["test_users", "users", "performance_test", self.table_name]
                for table in tables_to_clean:
                    try:
                        temp_bridge.drop_table(table, "mysql_cleanup")
                        print(f"✅ 已清理表: {table}")
                    except Exception as e:
                        print(f"⚠️ 清理表 {table} 时出错: {e}")
            else:
                print(f"⚠️ 无法连接到MySQL进行清理: {result_data.get('error')}")
                
        except Exception as e:
            print(f"⚠️ 清理过程中出错: {e}")
    
    def initialize(self) -> bool:
        """初始化测试环境"""
        print("🚀 初始化MySQL缓存性能对比测试环境...")
        
        try:
            # 创建测试数据目录
            os.makedirs(self.test_data_dir, exist_ok=True)
            
            # 创建数据库队列桥接器
            self.bridge = create_db_queue_bridge()
            self.add_database_connection(self.bridge)
            
            # 清理现有的测试表
            self._cleanup_existing_tables()
            
            # 添加带缓存的MySQL数据库
            self._add_cached_mysql_database()
            
            # 添加不带缓存的MySQL数据库
            self._add_non_cached_mysql_database()
            
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
        l1_config = PyL1CacheConfig(1000)  # 1000条记录
        l1_config.max_memory_mb = 100  # 100MB内存，适配系统限制
        l1_config.enable_stats = False  # 禁用统计以减少开销
        cache_config.l1_config = l1_config
        
        # L2缓存配置 - 合理的磁盘容量配置
        l2_config = PyL2CacheConfig(f"{self.test_data_dir}/mysql_cache_test")
        l2_config.max_disk_mb = 500  # 500MB磁盘空间
        l2_config.compression_level = 6  # 中等压缩级别
        l2_config.enable_wal = True  # 启用WAL
        l2_config.clear_on_startup = False  # 启动时不清空缓存目录
        cache_config.l2_config = l2_config
        
        # TTL配置 - 延长缓存时间确保测试期间不过期
        ttl_config = PyTtlConfig(300)  # 5分钟TTL
        ttl_config.max_ttl_secs = 3600  # 1小时最大TTL
        ttl_config.check_interval_secs = 60  # 1分钟检查间隔
        cache_config.ttl_config = ttl_config
        
        # 压缩配置 - 启用压缩
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = True  # 启用压缩
        compression_config.threshold_bytes = 1024
        cache_config.compression_config = compression_config
        
        print("  📊 缓存配置: L1(1000条/100MB) + L2(500MB) + TTL(5分钟) + ZSTD压缩")
        return cache_config
    
    def _add_cached_mysql_database(self):
        """添加带缓存的MySQL数据库"""
        cache_config = self._create_cached_config()
        
        response = self.bridge.add_mysql_database(
            alias="mysql_cached",
            host="172.16.0.21",
            port=3306,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            max_connections=10,
            min_connections=2,
            connection_timeout=30,  # 30秒连接超时
            idle_timeout=600,       # 10分钟空闲超时
            max_lifetime=3600,      # 1小时最大生命周期
            cache_config=cache_config
        )
        
        result = json.loads(response)
        if not result.get("success"):
            raise Exception(f"添加缓存MySQL数据库失败: {result.get('error')}")
    
    def _add_non_cached_mysql_database(self):
        """添加不带缓存的MySQL数据库"""
        # 真正的无缓存配置：不创建任何缓存管理器
        cache_config = None
        
        response = self.bridge.add_mysql_database(
            alias="mysql_non_cached",
            host="172.16.0.21",
            port=3306,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            max_connections=10,
            min_connections=2,
            connection_timeout=30,  # 30秒连接超时
            idle_timeout=600,       # 10分钟空闲超时
            max_lifetime=3600,      # 1小时最大生命周期
            cache_config=cache_config
        )
        
        result = json.loads(response)
        if not result.get("success"):
            raise Exception(f"添加非缓存MySQL数据库失败: {result.get('error')}")
    
    def setup_test_data(self) -> bool:
        """设置测试数据"""
        print("\n🔧 设置MySQL测试数据...")
        
        try:
            max_retries = 3  # 最大重试次数
            operation_timeout = 5  # 单个操作超时时间（秒）
            
            # 基础测试用户（为不同数据库使用不同的数据避免冲突）
            cached_users = [
                TestUser.new(i, f"缓存用户{i}", f"cached_user{i}@example.com", 20 + (i % 50), 
                           ["北京", "上海", "广州", "深圳", "杭州"][i % 5])
                for i in range(1, 1001)  # 1000条记录
            ]
            
            non_cached_users = [
                TestUser.new(i, f"非缓存用户{i}", f"non_cached_user{i}@example.com", 20 + (i % 50),
                           ["北京", "上海", "广州", "深圳", "杭州"][i % 5])
                for i in range(1, 1001)  # 1000条记录
            ]
            
            # 创建测试数据到缓存数据库
            for i, user in enumerate(cached_users):
                retry_count = 0
                success = False
                
                while retry_count < max_retries and not success:
                    try:
                        start_time = time.time()
                        response = self.bridge.create(self.table_name, user.to_json(), "mysql_cached")
                        
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
                        response = self.bridge.create(self.table_name, user.to_json(), "mysql_non_cached")
                        
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
            print(f"  📝 使用表名称: {self.table_name}")
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
            self.bridge.find(self.table_name, query_conditions_1, "mysql_cached")
            
            # 预热查询2 - 与test_repeated_queries中的查询条件完全一致
            query_conditions_2 = json.dumps([
                {"field": "city", "operator": "Eq", "value": "北京"},
                {"field": "age", "operator": "Gte", "value": 30}
            ])
            self.bridge.find(self.table_name, query_conditions_2, "mysql_cached")
            
            # 按ID查询预热 - 预热批量查询中会用到的ID
            for i in range(1, 21):
                query_conditions = json.dumps([
                    {"field": "id", "operator": "Eq", "value": i}
                ])
                self.bridge.find(self.table_name, query_conditions, "mysql_cached")
            
            # 预热年龄查询 - 预热批量查询中的年龄查询
            for i in range(1, 11):
                age_conditions = json.dumps([
                    {"field": "age", "operator": "Eq", "value": 20 + (i % 50)}
                ])
                self.bridge.find(self.table_name, age_conditions, "mysql_cached")
            
            print("  ✅ 缓存预热完成，预热了所有测试查询模式")
            print("  📊 预热内容: 2种复杂查询 + 20条ID查询 + 10种年龄查询")
            return True
            
        except Exception as e:
            print(f"❌ 缓存预热失败: {e}")
            return False
    
    def test_query_operations(self) -> bool:
        """测试查询操作性能"""
        print("\n🔍 测试MySQL查询操作性能...")
        
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
                self.bridge.find(self.table_name, query_conditions, "mysql_cached")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库查询（100次）
            start_time = time.time()
            for i in range(1, 101):
                self.bridge.find(self.table_name, query_conditions, "mysql_non_cached")
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 记录结果
            result = PerformanceResult.new(
                "复杂查询操作 (100次)",
                cached_duration,
                non_cached_duration
            )
            self.results.append(result)
            
            print(f"  ✅ 查询操作测试完成")
            print(f"  📊 缓存版本: {cached_duration:.1f}ms, 非缓存版本: {non_cached_duration:.1f}ms")
            print(f"  🚀 性能提升: {result.improvement_ratio:.1f}倍")
            return True
            
        except Exception as e:
            print(f"❌ 查询操作测试失败: {e}")
            return False
    
    def test_repeated_queries(self) -> bool:
        """测试重复查询性能（缓存命中）"""
        print("\n🔄 测试MySQL重复查询性能...")
        
        try:
            # 构建重复查询条件 - 查找北京且年龄>=30的用户
            query_conditions = json.dumps([
                {"field": "city", "operator": "Eq", "value": "北京"},
                {"field": "age", "operator": "Gte", "value": 30}
            ])
            
            # 测试缓存数据库重复查询（500次，大量重复查询体现缓存优势）
            start_time = time.time()
            for i in range(1, 501):
                self.bridge.find(self.table_name, query_conditions, "mysql_cached")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库重复查询（500次）
            start_time = time.time()
            for i in range(1, 501):
                self.bridge.find(self.table_name, query_conditions, "mysql_non_cached")
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 获取缓存统计信息
            try:
                cache_stats_response = self.bridge.get_cache_stats("mysql_cached")
                cache_stats = json.loads(cache_stats_response)
                if cache_stats.get("success"):
                    stats = cache_stats.get("stats", {})
                    hit_rate = stats.get("hit_rate", 0.0) * 100
                    print(f"  📈 缓存统计 - 命中: {stats.get('hits', 0)}, 未命中: {stats.get('misses', 0)}, 命中率: {hit_rate:.1f}%")
                else:
                    hit_rate = None
                    print(f"  ⚠️ 获取缓存统计失败: {cache_stats.get('error')}")
            except Exception as e:
                hit_rate = None
                print(f"  ⚠️ 获取缓存统计异常: {e}")
            
            # 记录结果
            result = PerformanceResult.new(
                "重复查询 (500次)",
                cached_duration,
                non_cached_duration
            )
            if hit_rate is not None:
                result = result.with_cache_hit_rate(hit_rate)
            self.results.append(result)
            
            print(f"  ✅ 重复查询测试完成")
            print(f"  📊 缓存版本: {cached_duration:.1f}ms, 非缓存版本: {non_cached_duration:.1f}ms")
            print(f"  🚀 性能提升: {result.improvement_ratio:.1f}倍")
            return True
            
        except Exception as e:
            print(f"❌ 重复查询测试失败: {e}")
            return False
    
    def test_batch_queries(self) -> bool:
        """测试批量查询性能"""
        print("\n📦 测试MySQL批量查询性能...")
        
        try:
            # 测试缓存数据库的批量查询（混合ID查询和范围查询）
            start_time = time.time()
            # 先查询一些具体ID（这些应该命中缓存）
            for i in range(1, 21):
                id_conditions = json.dumps([
                    {"field": "id", "operator": "Eq", "value": i}
                ])
                self.bridge.find(self.table_name, id_conditions, "mysql_cached")
            
            # 再查询一些年龄范围
            for i in range(1, 21):
                age_conditions = json.dumps([
                    {"field": "age", "operator": "Eq", "value": 20 + (i % 50)}
                ])
                self.bridge.find(self.table_name, age_conditions, "mysql_cached")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库的批量查询（相同的查询模式）
            start_time = time.time()
            # 查询相同的ID
            for i in range(1, 21):
                id_conditions = json.dumps([
                    {"field": "id", "operator": "Eq", "value": i}
                ])
                self.bridge.find(self.table_name, id_conditions, "mysql_non_cached")
            
            # 查询相同的年龄范围
            for i in range(1, 21):
                age_conditions = json.dumps([
                    {"field": "age", "operator": "Eq", "value": 20 + (i % 50)}
                ])
                self.bridge.find(self.table_name, age_conditions, "mysql_non_cached")
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 记录结果
            result = PerformanceResult.new(
                "批量查询 (20次ID查询 + 20次年龄查询)",
                cached_duration,
                non_cached_duration
            )
            self.results.append(result)
            
            print(f"  ✅ 批量查询测试完成")
            print(f"  📊 缓存版本: {cached_duration:.1f}ms, 非缓存版本: {non_cached_duration:.1f}ms")
            print(f"  🚀 性能提升: {result.improvement_ratio:.1f}倍")
            return True
            
        except Exception as e:
            print(f"❌ 批量查询测试失败: {e}")
            return False
    
    def test_update_operations(self) -> bool:
        """测试更新操作性能"""
        print("\n✏️ 测试MySQL更新操作性能...")
        
        try:
            # 构建更新数据
            update_data = json.dumps({"age": 30})
            
            # 测试缓存数据库的更新操作
            start_time = time.time()
            for i in range(1, 21):
                update_conditions = json.dumps([
                    {"field": "id", "operator": "Eq", "value": i}
                ])
                self.bridge.update(self.table_name, update_conditions, update_data, "mysql_cached")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 测试非缓存数据库的更新操作
            start_time = time.time()
            for i in range(1, 21):
                update_conditions = json.dumps([
                    {"field": "id", "operator": "Eq", "value": i}
                ])
                self.bridge.update(self.table_name, update_conditions, update_data, "mysql_non_cached")
            non_cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 记录结果
            result = PerformanceResult.new(
                "更新操作 (20次)",
                cached_duration,
                non_cached_duration
            )
            self.results.append(result)
            
            print(f"  ✅ 更新操作测试完成")
            print(f"  📊 缓存版本: {cached_duration:.1f}ms, 非缓存版本: {non_cached_duration:.1f}ms")
            print(f"  🚀 性能提升: {result.improvement_ratio:.1f}倍")
            return True
            
        except Exception as e:
            print(f"❌ 更新操作测试失败: {e}")
            return False
    
    def display_results(self):
        """显示测试结果"""
        print("\n" + "="*60)
        print("🎯 MySQL缓存性能测试结果汇总")
        print("="*60)
        
        total_cached_time = 0.0
        total_non_cached_time = 0.0
        
        for i, result in enumerate(self.results, 1):
            total_cached_time += result.with_cache
            total_non_cached_time += result.without_cache
            
            print(f"\n📊 测试 {i}: {result.operation}")
            print(f"   🟢 带缓存:   {result.with_cache:.1f} ms")
            print(f"   🔴 不带缓存: {result.without_cache:.1f} ms")
            print(f"   🚀 性能提升: {result.improvement_ratio:.1f}倍")
            
            if result.cache_hit_rate is not None:
                print(f"   📈 缓存命中率: {result.cache_hit_rate:.1f}%")
        
        # 总体统计
        print(f"\n" + "="*60)
        print("📈 总体性能统计")
        print("="*60)
        print(f"🟢 总缓存时间:   {total_cached_time:.1f} ms")
        print(f"🔴 总非缓存时间: {total_non_cached_time:.1f} ms")
        
        if total_cached_time > 0:
            overall_improvement = total_non_cached_time / total_cached_time
            print(f"🚀 总体性能提升: {overall_improvement:.1f}倍")
            
            time_saved = total_non_cached_time - total_cached_time
            time_saved_percent = (time_saved / total_non_cached_time) * 100
            print(f"⏱️ 节省时间: {time_saved:.1f} ms ({time_saved_percent:.1f}%)")
        
        print("="*60)
    
    def run_performance_test(self) -> bool:
        """运行完整的性能测试"""
        print("🚀 开始MySQL缓存性能对比测试")
        print(f"📝 RAT QuickDB 版本: {get_version()}")
        print(f"📋 测试信息: {get_info()}")
        
        try:
            # 初始化测试环境
            if not self.initialize():
                return False
            
            # 设置测试数据
            if not self.setup_test_data():
                return False
            
            # 预热缓存
            if not self.warmup_cache():
                return False
            
            # 运行各种性能测试
            if not self.test_query_operations():
                return False
            
            if not self.test_repeated_queries():
                return False
            
            if not self.test_batch_queries():
                return False
            
            if not self.test_update_operations():
                return False
            
            # 显示结果
            self.display_results()
            
            print("\n🎉 MySQL缓存性能测试完成！")
            return True
            
        except Exception as e:
            print(f"❌ 性能测试过程中发生错误: {e}")
            return False
    
    def cleanup_resources(self):
        """清理测试文件和数据（实现 GracefulShutdownMixin 的抽象方法）"""
        print("🧹 清理 MySQL 测试数据...")
        
        try:
            # 清理测试表数据，添加超时限制
            if self.bridge:
                try:
                    # 设置清理操作的超时时间
                    cleanup_start = time.time()
                    cleanup_timeout = 5  # 5秒超时
                    
                    # 删除缓存数据库中的测试数据
                    if time.time() - cleanup_start < cleanup_timeout:
                        delete_conditions = json.dumps([
                            {"field": "id", "operator": "Contains", "value": "cached_user_"}
                        ])
                        self.bridge.delete(self.table_name, delete_conditions, "mysql_cached")
                    
                    # 删除非缓存数据库中的测试数据
                    if time.time() - cleanup_start < cleanup_timeout:
                        delete_conditions = json.dumps([
                            {"field": "id", "operator": "Contains", "value": "non_cached_user_"}
                        ])
                        self.bridge.delete(self.table_name, delete_conditions, "mysql_non_cached")
                    
                    print(f"  ✅ 已清理MySQL测试表: {self.table_name}")
                except Exception as e:
                    print(f"  ⚠️  清理MySQL测试数据失败: {e}")
            
            print("✅ MySQL 测试数据清理完成")
            
        except Exception as e:
            print(f"❌ 清理过程中发生错误: {e}")
    
    def cleanup(self):
        """兼容性方法，调用优雅关闭"""
        self.shutdown()


def main():
    """主函数"""
    # 注册信号处理器
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    global test_instance
    test_instance = MySqlCachePerformanceTest()
    
    try:
        success = test_instance.run_performance_test()
        if success:
            print("✅ 测试成功完成")
            exit_code = 0
        else:
            print("❌ 测试失败")
            exit_code = 1
    except KeyboardInterrupt:
        print("\n🛑 用户中断测试")
        exit_code = 130
    except Exception as e:
        print(f"❌ 测试过程中发生未预期错误: {e}")
        exit_code = 1
    finally:
        # 清理资源
        if test_instance:
            test_instance.shutdown()
    
    exit(exit_code)


if __name__ == "__main__":
    main()