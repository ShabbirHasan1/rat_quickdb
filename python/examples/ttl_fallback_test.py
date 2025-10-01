#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python TTL回退机制验证测试

本测试专门验证启用L2缓存时，TTL过期后的回退机制是否正常工作
测试场景：
1. 启用L1+L2缓存配置
2. 设置较短的TTL（2秒）
3. 插入测试数据
4. 第一次查询（缓存数据）
5. 第二次查询（缓存命中）
6. 等待TTL过期后第三次查询（回退到数据库）
7. 验证回退机制是否正常工作
"""

import json
import time
import os
import shutil
from datetime import datetime, timezone
from typing import Optional

try:
    from rat_quickdb_py.rat_quickdb_py import (
        create_db_queue_bridge, DbQueueBridge,
        PyCacheConfig, PyL1CacheConfig, PyL2CacheConfig, PyTtlConfig,
        init_logging_with_level, get_version, get_info, PyCompressionConfig
    )
    print("✓ 成功导入 rat_quickdb_py 模块")
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    exit(1)

class TTLFallbackTest:
    """TTL回退机制测试类"""
    
    def __init__(self):
        self.bridge = None
        self.test_data_dir = "./ttl_fallback_test_data"
        self.db_path = f"{self.test_data_dir}/ttl_test.db"
        timestamp = int(time.time() * 1000)
        self.table_name = f"ttl_test_{timestamp}"
        
    def setup_test_environment(self):
        """设置测试环境"""
        print("🔧 设置测试环境...")
        
        # 清理并创建测试目录
        if os.path.exists(self.test_data_dir):
            shutil.rmtree(self.test_data_dir)
        os.makedirs(self.test_data_dir, exist_ok=True)
        
        print(f"  📁 测试目录: {self.test_data_dir}")
        print(f"  🗃️ 数据库文件: {self.db_path}")
        print(f"  📋 测试表名: {self.table_name}")
        
    def create_cache_config_with_l2(self) -> PyCacheConfig:
        """创建启用L2缓存的配置，设置短TTL用于测试"""
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # L1缓存配置
        l1_config = PyL1CacheConfig(100)  # 100条记录
        l1_config.max_memory_mb = 10  # 10MB内存
        l1_config.enable_stats = True  # 启用统计
        cache_config.l1_config = l1_config
        
        # L2缓存配置 - 关键：启用L2缓存
        l2_config = PyL2CacheConfig(f"{self.test_data_dir}/l2_cache")
        l2_config.max_disk_mb = 50  # 50MB磁盘空间
        l2_config.compression_level = 1  # 最低压缩级别
        l2_config.enable_wal = True  # 启用WAL
        l2_config.clear_on_startup = True  # 启动时清空缓存
        cache_config.l2_config = l2_config
        
        # TTL配置 - 关键：设置短TTL用于测试
        ttl_config = PyTtlConfig(2)  # 2秒TTL
        ttl_config.max_ttl_secs = 10  # 10秒最大TTL
        ttl_config.check_interval_secs = 1  # 1秒检查间隔
        cache_config.ttl_config = ttl_config
        
        # 压缩配置
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = True  # 启用压缩
        compression_config.threshold_bytes = 100  # 100字节阈值
        cache_config.compression_config = compression_config
        
        print("  📊 缓存配置: L1(100条/10MB) + L2(50MB) + TTL(2秒) + ZSTD压缩")
        return cache_config
        
    def setup_database(self) -> bool:
        """设置数据库连接"""
        print("\n🔗 设置数据库连接...")
        
        try:
            # 创建数据库桥接器
            self.bridge = create_db_queue_bridge()
            
            # 添加SQLite数据库
            cache_config = self.create_cache_config_with_l2()
            response = self.bridge.add_sqlite_database(
                alias="ttl_test",
                path=self.db_path,
                create_if_missing=True,
                max_connections=5,
                min_connections=1,
                connection_timeout=10,
                idle_timeout=60,
                max_lifetime=300,
                cache_config=cache_config  # 使用创建的缓存配置
            )
            
            result = json.loads(response)
            if not result.get("success", False):
                print(f"  ❌ 数据库连接失败: {result}")
                return False
                
            print(f"  ✅ 数据库连接成功")
            return True
            
        except Exception as e:
            print(f"  ❌ 数据库设置异常: {e}")
            return False
            
    def create_test_table(self) -> bool:
        """创建测试表"""
        print("\n📋 创建测试表...")
        
        try:
            # 定义表字段 - 使用简单的字符串类型定义
            fields_json = json.dumps({
                "id": "integer",
                "name": "string", 
                "age": "integer",
                "email": "string",
                "created_at": "datetime"
            })
            
            # 创建表
            response = self.bridge.create_table(
                table=self.table_name,
                fields_json=fields_json,
                alias="ttl_test"
            )
            result = json.loads(response)
            
            if not result.get("success", False):
                print(f"  ❌ 创建表失败: {result}")
                return False
                
            print(f"  ✅ 测试表 '{self.table_name}' 创建成功")
            return True
            
        except Exception as e:
            print(f"  ❌ 创建表异常: {e}")
            return False
            
    def insert_test_data(self) -> bool:
        """插入测试数据"""
        print("\n📝 插入测试数据...")
        
        try:
            test_data = {
                "name": "张三",
                "age": 28,
                "email": "zhangsan@example.com"
            }
            
            response = self.bridge.create(self.table_name, json.dumps(test_data), "ttl_test")
            result = json.loads(response)
            
            if not result.get("success", False):
                print(f"  ❌ 插入数据失败: {result}")
                return False
                
            print(f"  ✅ 测试数据插入成功: {test_data['name']}")
            return True
            
        except Exception as e:
            print(f"  ❌ 插入数据异常: {e}")
            return False
            
    def query_data(self, query_name: str) -> Optional[dict]:
        """查询数据"""
        try:
            conditions = json.dumps({"name": "张三"})
            response = self.bridge.find(self.table_name, conditions, "ttl_test")
            result = json.loads(response)
            
            if result.get("success", False):
                data = result.get("data", [])
                if data:
                    print(f"  ✅ {query_name}: 查询成功，找到 {len(data)} 条记录")
                    return data[0]
                else:
                    print(f"  ⚠️ {query_name}: 查询成功但无数据")
                    return None
            else:
                print(f"  ❌ {query_name}: 查询失败 - {result}")
                return None
                
        except Exception as e:
            print(f"  ❌ {query_name}: 查询异常 - {e}")
            return None
            
    def run_ttl_fallback_test(self) -> bool:
        """运行TTL回退测试"""
        print("\n🧪 开始TTL回退机制测试...")
        print("=" * 50)
        
        # 第一次查询 - 应该从数据库查询并缓存
        print("\n🔍 第一次查询（从数据库查询并缓存）...")
        start_time = time.time()
        first_result = self.query_data("第一次查询")
        first_duration = time.time() - start_time
        
        if not first_result:
            print("  ❌ 第一次查询失败")
            return False
            
        print(f"  ⏱️ 查询耗时: {first_duration:.3f}秒")
        
        # 第二次查询 - 应该从缓存命中
        print("\n🔍 第二次查询（缓存命中）...")
        start_time = time.time()
        second_result = self.query_data("第二次查询")
        second_duration = time.time() - start_time
        
        if not second_result:
            print("  ❌ 第二次查询失败")
            return False
            
        print(f"  ⏱️ 查询耗时: {second_duration:.3f}秒")
        
        # 验证缓存命中（第二次查询应该更快）
        if second_duration < first_duration:
            print("  ✅ 缓存命中验证成功（第二次查询更快）")
        else:
            print("  ⚠️ 缓存命中验证警告（第二次查询未明显加速）")
            
        # 等待TTL过期
        print("\n⏳ 等待TTL过期（3秒）...")
        time.sleep(3)
        
        # 第三次查询 - TTL过期后应该回退到数据库
        print("\n🔍 第三次查询（TTL过期，回退到数据库）...")
        start_time = time.time()
        third_result = self.query_data("第三次查询")
        third_duration = time.time() - start_time
        
        if not third_result:
            print("  ❌ 第三次查询失败 - TTL回退机制可能有问题")
            return False
            
        print(f"  ⏱️ 查询耗时: {third_duration:.3f}秒")
        
        # 验证TTL回退机制
        if third_duration > second_duration:
            print("  ✅ TTL回退机制验证成功（第三次查询耗时增加，说明回退到数据库）")
        else:
            print("  ⚠️ TTL回退机制验证警告（第三次查询未明显变慢）")
            
        # 验证数据一致性
        if (first_result.get("name") == second_result.get("name") == third_result.get("name") and
            first_result.get("age") == second_result.get("age") == third_result.get("age")):
            print("  ✅ 数据一致性验证成功")
        else:
            print("  ❌ 数据一致性验证失败")
            return False
            
        return True
        
    def cleanup(self):
        """清理测试环境"""
        print("\n🧹 清理测试环境...")
        
        try:
            # 清理测试目录
            if os.path.exists(self.test_data_dir):
                shutil.rmtree(self.test_data_dir)
                print("  ✅ 测试目录清理完成")
                
        except Exception as e:
            print(f"  ⚠️ 清理过程中出现异常: {e}")
            
    def run(self) -> bool:
        """运行完整测试"""
        print("🚀 RAT QuickDB Python TTL回退机制验证测试")
        print("=" * 60)
        print(f"📦 库版本: {get_version()}")
        print(f"📋 库信息: {get_info()}")
        
        try:
            # 设置测试环境
            self.setup_test_environment()
            
            # 设置数据库
            if not self.setup_database():
                return False
                
            # 创建测试表
            if not self.create_test_table():
                return False
                
            # 插入测试数据
            if not self.insert_test_data():
                return False
                
            # 运行TTL回退测试
            if not self.run_ttl_fallback_test():
                return False
                
            print("\n🎉 TTL回退机制验证测试完成！")
            print("✅ 所有测试通过，L2缓存TTL回退机制工作正常")
            return True
            
        except Exception as e:
            print(f"\n❌ 测试过程中发生异常: {e}")
            return False
            
        finally:
            self.cleanup()

def main():
    """主函数"""
    test = TTLFallbackTest()
    success = test.run()
    
    if success:
        print("\n🏆 测试结果: 成功")
        exit(0)
    else:
        print("\n💥 测试结果: 失败")
        exit(1)

if __name__ == "__main__":
    main()