#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python Bridge实例测试

测试使用同一个bridge实例添加多个数据库时的行为
对比单独bridge实例和共享bridge实例的差异
"""

import json
import time
import os
from typing import Optional

try:
    import rat_quickdb_py
    from rat_quickdb_py import (
        create_db_queue_bridge,
        get_version,
        PyCacheConfig,
        PyL1CacheConfig,
        PyTtlConfig,
        PyCompressionConfig,
        PyTlsConfig,
        PyZstdConfig,
    )
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    exit(1)


class BridgeInstanceTest:
    """Bridge实例测试"""
    
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
    
    def create_cache_config(self) -> PyCacheConfig:
        """创建标准缓存配置"""
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.strategy = "lru"
        
        # 简化的L1缓存配置
        l1_config = PyL1CacheConfig(1000)
        l1_config.max_memory_mb = 100
        l1_config.enable_stats = False
        cache_config.l1_config = l1_config
        
        # 简化的TTL配置
        ttl_config = PyTtlConfig(300)  # 5分钟TTL
        ttl_config.max_ttl_secs = 1800   # 30分钟最大TTL
        ttl_config.check_interval_secs = 60  # 1分钟检查间隔
        cache_config.ttl_config = ttl_config
        
        # 禁用压缩以简化配置
        compression_config = PyCompressionConfig("zstd")
        compression_config.enabled = False
        compression_config.threshold_bytes = 1024
        cache_config.compression_config = compression_config
        
        return cache_config
    
    def create_tls_config(self) -> PyTlsConfig:
        """创建标准TLS配置"""
        tls_config = PyTlsConfig()
        tls_config.enable()
        
        ca_cert_path = self.get_ca_cert_path()
        if ca_cert_path:
            tls_config.ca_cert_path = ca_cert_path
        
        tls_config.client_cert_path = ""
        tls_config.client_key_path = ""
        
        return tls_config
    
    def create_zstd_config(self, enabled: bool = False) -> PyZstdConfig:
        """创建ZSTD配置"""
        zstd_config = PyZstdConfig()
        if enabled:
            zstd_config.enable()
            zstd_config.compression_level = 3
            zstd_config.compression_threshold = 1024
        else:
            zstd_config.disable()
        
        return zstd_config
    
    def add_database_with_cache(self, bridge, alias: str) -> bool:
        """添加带缓存的数据库"""
        try:
            cache_config = self.create_cache_config()
            tls_config = self.create_tls_config()
            zstd_config = self.create_zstd_config(False)
            
            print(f"📋 添加缓存数据库: {alias}")
            print(f"   缓存配置: enabled={cache_config.enabled}")
            
            response = bridge.add_mongodb_database(
                alias=alias,
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
                zstd_config=zstd_config
            )
            
            result = json.loads(response)
            if not result.get("success"):
                print(f"❌ 添加缓存数据库失败: {result.get('error')}")
                return False
            
            print(f"✅ 缓存数据库添加成功: {alias}")
            return True
            
        except Exception as e:
            print(f"❌ 添加缓存数据库异常: {e}")
            return False
    
    def add_database_without_cache(self, bridge, alias: str) -> bool:
        """添加不带缓存的数据库"""
        try:
            cache_config = None
            tls_config = self.create_tls_config()
            zstd_config = self.create_zstd_config(False)
            
            print(f"📋 添加非缓存数据库: {alias}")
            print(f"   缓存配置: {cache_config}")
            
            response = bridge.add_mongodb_database(
                alias=alias,
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
                cache_config=cache_config,  # 不启用缓存
                tls_config=tls_config,
                zstd_config=zstd_config
            )
            
            result = json.loads(response)
            if not result.get("success"):
                print(f"❌ 添加非缓存数据库失败: {result.get('error')}")
                return False
            
            print(f"✅ 非缓存数据库添加成功: {alias}")
            return True
            
        except Exception as e:
            print(f"❌ 添加非缓存数据库异常: {e}")
            return False
    
    def trigger_adapter_creation(self, bridge, alias: str):
        """触发适配器创建"""
        try:
            print(f"🔍 触发 {alias} 适配器创建...")
            bridge.find_by_id("dummy_collection", "dummy_id", alias)
        except:
            pass  # 忽略查询错误，只关注适配器创建日志
    
    def test_shared_bridge_instance(self) -> bool:
        """测试共享bridge实例（模拟原始测试）"""
        print("\n🧪 测试场景：共享Bridge实例（模拟原始测试）")
        print("=" * 50)
        
        try:
            # 创建单个bridge实例
            bridge = create_db_queue_bridge()
            
            # 添加缓存数据库
            if not self.add_database_with_cache(bridge, "shared_cached"):
                return False
            
            # 添加非缓存数据库
            if not self.add_database_without_cache(bridge, "shared_non_cached"):
                return False
            
            # 触发适配器创建
            print("\n🔍 触发适配器创建...")
            self.trigger_adapter_creation(bridge, "shared_cached")
            self.trigger_adapter_creation(bridge, "shared_non_cached")
            
            print("✅ 共享Bridge实例测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 共享Bridge实例测试失败: {e}")
            return False
    
    def test_separate_bridge_instances(self) -> bool:
        """测试独立bridge实例"""
        print("\n🧪 测试场景：独立Bridge实例")
        print("=" * 50)
        
        try:
            # 为缓存数据库创建独立bridge实例
            bridge1 = create_db_queue_bridge()
            if not self.add_database_with_cache(bridge1, "separate_cached"):
                return False
            
            print("\n🔍 触发缓存数据库适配器创建...")
            self.trigger_adapter_creation(bridge1, "separate_cached")
            
            # 为非缓存数据库创建独立bridge实例
            bridge2 = create_db_queue_bridge()
            if not self.add_database_without_cache(bridge2, "separate_non_cached"):
                return False
            
            print("\n🔍 触发非缓存数据库适配器创建...")
            self.trigger_adapter_creation(bridge2, "separate_non_cached")
            
            print("✅ 独立Bridge实例测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 独立Bridge实例测试失败: {e}")
            return False
    
    def test_timing_issue(self) -> bool:
        """测试时序问题"""
        print("\n🧪 测试场景：时序问题（添加间隔）")
        print("=" * 50)
        
        try:
            # 创建单个bridge实例
            bridge = create_db_queue_bridge()
            
            # 添加缓存数据库
            if not self.add_database_with_cache(bridge, "timing_cached"):
                return False
            
            # 等待一段时间
            print("⏰ 等待2秒...")
            time.sleep(2)
            
            # 添加非缓存数据库
            if not self.add_database_without_cache(bridge, "timing_non_cached"):
                return False
            
            # 等待一段时间
            print("⏰ 等待2秒...")
            time.sleep(2)
            
            # 触发适配器创建
            print("\n🔍 触发适配器创建...")
            self.trigger_adapter_creation(bridge, "timing_cached")
            time.sleep(1)
            self.trigger_adapter_creation(bridge, "timing_non_cached")
            
            print("✅ 时序问题测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 时序问题测试失败: {e}")
            return False
    
    def cleanup_existing_collections(self):
        """清理现有的测试集合"""
        print("🧹 清理现有的测试集合...")
        
        try:
            # 创建临时bridge用于清理
            temp_bridge = create_db_queue_bridge()
            
            # 尝试添加数据库连接进行清理
            cache_config = self.create_cache_config()
            tls_config = self.create_tls_config()
            zstd_config = self.create_zstd_config(False)
            
            # 添加临时数据库连接
            try:
                response = temp_bridge.add_mongodb_database(
                    alias="cleanup_db",
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
                    cache_config=None,
                    tls_config=tls_config,
                    zstd_config=zstd_config
                )
                
                result = json.loads(response)
                if result.get("success"):
                    # 删除可能存在的测试集合数据
                    collections_to_clean = ["dummy_collection", "test_collection", "users"]
                    
                    for collection in collections_to_clean:
                        try:
                            delete_conditions = json.dumps([])
                            temp_bridge.delete(collection, delete_conditions, "cleanup_db")
                            print(f"  ✅ 已清理集合: {collection}")
                        except Exception as e:
                            print(f"  ⚠️ 清理集合 {collection} 失败（可能不存在）: {e}")
                            
            except Exception as e:
                print(f"  ⚠️ 添加清理数据库连接失败: {e}")
                
        except Exception as e:
            print(f"  ⚠️ 清理测试集合过程中发生错误: {e}")
    
    def run_all_tests(self) -> int:
        """运行所有测试"""
        print("🚀 RatQuickDB Python Bridge实例测试")
        print("====================================")
        
        try:
            print(f"📦 RatQuickDB 版本: {get_version()}")
        except Exception as e:
            print(f"⚠️ 无法获取版本信息: {e}")
        
        # 清理现有的测试数据
        self.cleanup_existing_collections()
        
        success_count = 0
        total_tests = 3
        
        try:
            # 测试1：共享Bridge实例
            if self.test_shared_bridge_instance():
                success_count += 1
            
            # 测试2：独立Bridge实例
            if self.test_separate_bridge_instances():
                success_count += 1
            
            # 测试3：时序问题
            if self.test_timing_issue():
                success_count += 1
            
            # 总结测试结果
            print("\n📊 测试结果总结")
            print("=" * 40)
            print(f"✅ 成功测试: {success_count}/{total_tests}")
            print(f"❌ 失败测试: {total_tests - success_count}/{total_tests}")
            
            if success_count == total_tests:
                print("🎯 所有测试场景都已完成！")
                print("\n🔍 请对比不同场景下的适配器类型：")
                print("   - 共享Bridge实例 vs 独立Bridge实例")
                print("   - 是否存在时序相关的问题")
                return 0
            else:
                print("❌ 部分测试失败，请检查错误信息")
                return 1
            
        except Exception as e:
            print(f"❌ 测试执行失败: {e}")
            return 1


def main() -> int:
    """主函数"""
    test = BridgeInstanceTest()
    
    try:
        return test.run_all_tests()
    except KeyboardInterrupt:
        print("\n🛑 收到键盘中断，退出测试")
        return 0
    except Exception as e:
        print(f"⚠️ 程序执行出错: {e}")
        return 1


if __name__ == "__main__":
    exit(main())