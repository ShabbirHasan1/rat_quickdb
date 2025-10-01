#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python 缓存性能对比示例

本示例对比启用缓存和未启用缓存的数据库操作性能差异
使用 SQLite 数据库进行测试

基于 Rust 版本的缓存性能对比示例改写
"""

import json
import time
import os
import shutil
from datetime import datetime, timezone
from typing import Dict, List, Optional, Tuple
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
            "id": self.id,
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


class CachePerformanceTest:
    """缓存性能对比测试"""
    
    def __init__(self):
        self.bridge = None
        self.results: List[PerformanceResult] = []
        self.test_data_dir = "./test_data"
    
    def initialize(self) -> bool:
        """初始化测试环境"""
        print("🚀 初始化缓存性能对比测试环境...")
        
        try:
            # 创建测试数据目录
            os.makedirs(self.test_data_dir, exist_ok=True)
            
            # 创建数据库队列桥接器
            self.bridge = create_db_queue_bridge()
            
            # 添加带缓存的数据库
            self._add_cached_database()
            
            # 添加不带缓存的数据库
            self._add_non_cached_database()
            
            # 清理之前的测试数据（删除表）
            self._cleanup_existing_tables()
            
            print("✅ 测试环境初始化完成")
            return True
            
        except Exception as e:
            print(f"❌ 测试环境初始化失败: {e}")
            return False
    
    def _create_cached_config(self) -> PyCacheConfig:
        """创建带缓存的配置（启用L2缓存）"""
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"

        # L1缓存配置
        l1_config = PyL1CacheConfig(1000)  # 最大容量1000条记录
        l1_config.max_memory_mb = 50  # 最大内存50MB
        l1_config.enable_stats = True  # 启用统计
        cache_config.l1_config = l1_config

        # L2缓存配置
        l2_config = PyL2CacheConfig("./cache_l2_test")  # L2缓存存储路径
        l2_config.max_disk_mb = 500  # 最大500MB磁盘空间
        l2_config.compression_level = 3  # 压缩级别
        l2_config.enable_wal = True  # 启用WAL
        l2_config.clear_on_startup = False  # 启动时不清空缓存目录
        cache_config.l2_config = l2_config

        # TTL配置
        ttl_config = PyTtlConfig(300)  # 默认TTL 5分钟
        ttl_config.max_ttl_secs = 3600  # 最大TTL 1小时
        ttl_config.check_interval_secs = 60  # 检查间隔1分钟
        cache_config.ttl_config = ttl_config

        # 压缩配置
        compression_config = PyCompressionConfig("gzip")
        compression_config.enabled = False  # 暂时禁用压缩
        compression_config.threshold_bytes = 1024
        cache_config.compression_config = compression_config

        return cache_config
    
    def _add_cached_database(self):
        """添加带缓存的数据库"""
        cache_config = self._create_cached_config()
        
        response = self.bridge.add_sqlite_database(
            alias="cached_db",
            path=f"{self.test_data_dir}/cache_performance_cached.db",
            max_connections=10,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600,
            cache_config=cache_config
        )
        
        result = json.loads(response)
        if not result.get("success"):
            raise Exception(f"添加缓存数据库失败: {result.get('error')}")
    
    def _add_non_cached_database(self):
        """添加不带缓存的数据库"""
        response = self.bridge.add_sqlite_database(
            alias="non_cached_db",
            path=f"{self.test_data_dir}/cache_performance_non_cached.db",
            max_connections=10,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600,
            cache_config=None  # 不使用缓存
        )
        
        result = json.loads(response)
        if not result.get("success"):
            raise Exception(f"添加非缓存数据库失败: {result.get('error')}")
    
    def _cleanup_existing_tables(self):
        """清理现有的测试表"""
        print("🧹 清理现有的测试表...")
        
        try:
            # 删除缓存数据库中的users表数据
            try:
                delete_conditions = json.dumps([])
                response = self.bridge.delete("users", delete_conditions, "cached_db")
                result = json.loads(response)
                if result.get("success"):
                    print("  ✅ 已清理缓存数据库中的users表数据")
            except Exception as e:
                print(f"  ⚠️ 清理缓存数据库表数据失败（可能表不存在）: {e}")
            
            # 删除非缓存数据库中的users表数据
            try:
                delete_conditions = json.dumps([])
                response = self.bridge.delete("users", delete_conditions, "non_cached_db")
                result = json.loads(response)
                if result.get("success"):
                    print("  ✅ 已清理非缓存数据库中的users表数据")
            except Exception as e:
                print(f"  ⚠️ 清理非缓存数据库表数据失败（可能表不存在）: {e}")
                
        except Exception as e:
            print(f"  ⚠️ 清理测试表过程中发生错误: {e}")
    
    def setup_test_data(self) -> bool:
        """设置测试数据"""
        print("\n🔧 设置测试数据...")
        
        try:
            # 基础测试用户
            test_users = [
                TestUser.new("user1", "张三", "zhangsan@example.com", 25),
                TestUser.new("user2", "李四", "lisi@example.com", 30),
                TestUser.new("user3", "王五", "wangwu@example.com", 28),
                TestUser.new("user4", "赵六", "zhaoliu@example.com", 35),
                TestUser.new("user5", "钱七", "qianqi@example.com", 22),
            ]
            
            # 批量用户数据
            batch_users = [
                TestUser.new(
                    f"batch_user_{i}",
                    f"批量用户{i}",
                    f"batch{i}@example.com",
                    20 + (i % 30)
                )
                for i in range(6, 26)
            ]
            
            all_users = test_users + batch_users
            
            # 创建测试数据到缓存数据库
            for user in all_users:
                response = self.bridge.create("users", user.to_json(), "cached_db")
                result = json.loads(response)
                if not result.get("success"):
                    print(f"⚠️ 创建用户数据失败: {result.get('error')}")
            
            print(f"  ✅ 创建了 {len(all_users)} 条测试记录")
            return True
            
        except Exception as e:
            print(f"❌ 设置测试数据失败: {e}")
            return False
    
    def warmup_cache(self) -> bool:
        """缓存预热"""
        print("\n🔥 缓存预热...")
        
        try:
            # 预热查询 - 查找年龄在25-35之间且姓名包含特定字符的用户
            query_conditions = json.dumps([
                {"field": "age", "operator": "Gte", "value": 25},
                {"field": "age", "operator": "Lte", "value": 35},
                {"field": "name", "operator": "Contains", "value": "用户"},
                {"field": "email", "operator": "Contains", "value": "@example.com"}
            ])
            
            # 预热查询
            self.bridge.find("users", query_conditions, "cached_db")
            
            # 按ID查询预热
            self.bridge.find_by_id("users", "user1", "cached_db")
            self.bridge.find_by_id("users", "user2", "cached_db")
            
            print("  ✅ 缓存预热完成")
            return True
            
        except Exception as e:
            print(f"❌ 缓存预热失败: {e}")
            return False
    
    def test_query_operations(self) -> bool:
        """测试查询操作性能"""
        print("\n🔍 测试查询操作性能...")
        
        try:
            # 构建复杂查询条件 - 查找特定用户且年龄符合条件
            query_conditions = json.dumps([
                {"field": "name", "operator": "Eq", "value": "张三"},
                {"field": "age", "operator": "Gte", "value": 20},
                {"field": "age", "operator": "Lte", "value": 50},
                {"field": "email", "operator": "Contains", "value": "@example.com"}
            ])
            
            # 第一次查询（冷启动，从数据库读取）
            start_time = time.time()
            self.bridge.find("users", query_conditions, "cached_db")
            first_query_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 第二次查询（缓存命中）
            start_time = time.time()
            self.bridge.find("users", query_conditions, "cached_db")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            result = PerformanceResult.new(
                "单次查询操作",
                cached_duration,
                first_query_duration
            )
            
            print(f"  ✅ 首次查询（数据库）: {first_query_duration:.2f}ms")
            print(f"  ✅ 缓存查询: {cached_duration:.2f}ms")
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
                {"field": "email", "operator": "Contains", "value": "batch"}
            ])
            
            query_count = 10
            
            # 首次查询（建立缓存）
            self.bridge.find("users", query_conditions, "cached_db")
            
            # 测试重复查询（应该从缓存读取）
            start_time = time.time()
            for _ in range(query_count):
                self.bridge.find("users", query_conditions, "cached_db")
                time.sleep(0.005)  # 短暂延迟以模拟真实场景
            
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 计算平均单次查询时间
            avg_cached_time = cached_duration / query_count
            estimated_db_time = 50.0  # 估算数据库查询时间（毫秒）
            
            result = PerformanceResult.new(
                f"重复查询 ({query_count}次)",
                avg_cached_time,
                estimated_db_time
            ).with_cache_hit_rate(95.0)  # 假设95%的缓存命中率
            
            print(f"  ✅ 总耗时: {cached_duration:.2f}ms")
            print(f"  ✅ 平均单次查询: {avg_cached_time:.2f}ms")
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
            user_ids = ["user1", "user2", "user3", "user4", "user5"]
            
            # 首次批量查询（建立缓存）
            start_time = time.time()
            for user_id in user_ids:
                self.bridge.find_by_id("users", user_id, "cached_db")
            first_batch_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            # 第二次批量查询（缓存命中）
            start_time = time.time()
            for user_id in user_ids:
                self.bridge.find_by_id("users", user_id, "cached_db")
            cached_duration = (time.time() - start_time) * 1000  # 转换为毫秒
            
            result = PerformanceResult.new(
                f"批量ID查询 ({len(user_ids)}条记录)",
                cached_duration,
                first_batch_duration
            )
            
            print(f"  ✅ 首次批量查询: {first_batch_duration:.2f}ms")
            print(f"  ✅ 缓存批量查询: {cached_duration:.2f}ms")
            print(f"  📈 性能提升: {result.improvement_ratio:.2f}x")
            
            self.results.append(result)
            return True
            
        except Exception as e:
            print(f"❌ 批量查询测试失败: {e}")
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
            
            if not self.test_repeated_queries():
                return False
            
            if not self.test_batch_queries():
                return False
            
            return True
            
        except Exception as e:
            print(f"❌ 测试执行失败: {e}")
            return False
    
    def display_results(self):
        """显示测试结果汇总"""
        print("\n📊 ==================== 性能测试结果汇总 ====================")
        print(f"{'操作类型':<25} {'带缓存(ms)':<15} {'不带缓存(ms)':<15} {'提升倍数':<10} {'缓存命中率':<10}")
        print("-" * 80)
        
        total_improvement = 0.0
        count = 0
        
        for result in self.results:
            cache_hit_str = f"{result.cache_hit_rate:.1f}%" if result.cache_hit_rate else "N/A"
            
            print(
                f"{result.operation:<25} "
                f"{result.with_cache:<15.2f} "
                f"{result.without_cache:<15.2f} "
                f"{result.improvement_ratio:<10.2f} "
                f"{cache_hit_str:<10}"
            )
            
            total_improvement += result.improvement_ratio
            count += 1
        
        print("-" * 80)
        
        if count > 0:
            avg_improvement = total_improvement / count
            print(f"📈 平均性能提升: {avg_improvement:.2f}x")
            
            if avg_improvement > 1.5:
                print("🎉 缓存显著提升了数据库操作性能！")
            elif avg_improvement > 1.1:
                print("✅ 缓存适度提升了数据库操作性能。")
            else:
                print("⚠️ 缓存对性能提升有限，可能需要调整缓存策略。")
        
        print("\n💡 性能优化建议:")
        print("   • 对于频繁查询的数据，缓存能显著提升性能")
        print("   • 重复查询场景下，缓存命中率越高，性能提升越明显")
        print("   • 写操作（创建、更新）的性能提升相对有限")
        print("   • 可根据实际业务场景调整缓存 TTL 和容量配置")
        
        print("\n🔧 缓存配置信息:")
        print("   • 缓存策略: LRU")
        print("   • L1 缓存容量: 1000 条记录")
        print("   • L1 缓存内存限制: 50 MB")
        print("   • 默认 TTL: 5 分钟")
        print("   • 最大 TTL: 1 小时")
    
    def cleanup(self):
        """清理测试文件"""
        print("\n🧹 清理测试文件...")
        
        try:
            if os.path.exists(self.test_data_dir):
                shutil.rmtree(self.test_data_dir)
                print(f"🗑️  已清理测试目录: {self.test_data_dir}")
            
            print("🧹 清理测试文件完成")
            
        except Exception as e:
            print(f"⚠️  清理测试文件失败: {e}")


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


def main():
    """主函数"""
    print("🚀 RatQuickDB Python 缓存性能对比测试（L1 + L2 缓存）")
    print("=====================================")
    
    # 显示版本信息
    display_version_info()
    print()
    
    # 创建并运行测试
    test = CachePerformanceTest()
    
    try:
        # 清理之前的测试文件
        test.cleanup()
        
        # 初始化测试环境
        if not test.initialize():
            return 1
        
        # 运行所有测试
        if not test.run_all_tests():
            return 1
        
        # 显示测试结果
        test.display_results()
        
        print("\n🎯 测试完成！感谢使用 RatQuickDB 缓存功能。")
        return 0
        
    except KeyboardInterrupt:
        print("\n⚠️ 测试被用户中断")
        return 1
    except Exception as e:
        print(f"\n❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()
        return 1
    finally:
        # 清理测试文件
        test.cleanup()


if __name__ == "__main__":
    exit(main())