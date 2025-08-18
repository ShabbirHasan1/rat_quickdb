#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python 缓存配置传递调试测试

专门用于调试Python绑定中缓存配置传递问题的简化测试
重点验证：
1. 单独缓存模式 - 应该使用缓存适配器
2. 单独非缓存模式 - 应该使用普通适配器  
3. 混合模式 - 两种适配器并存
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


class CacheConfigDebugTest:
    """缓存配置传递调试测试"""
    
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
        self.test_data_dir = "./cache_debug_test_data"
        timestamp = int(time.time() * 1000)
        self.collection_name = f"debug_test_{timestamp}"
    
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
    
    def test_scenario_1_cache_only(self) -> bool:
        """测试场景1：单独缓存模式"""
        print("\n🧪 测试场景1：单独缓存模式")
        print("=" * 40)
        
        try:
            # 创建数据库队列桥接器
            bridge = create_db_queue_bridge()
            
            # 创建缓存配置
            cache_config = self.create_cache_config()
            tls_config = self.create_tls_config()
            zstd_config = self.create_zstd_config(False)
            
            print(f"📋 缓存配置状态: enabled={cache_config.enabled}")
            print(f"📋 缓存策略: {cache_config.strategy}")
            print(f"📋 L1缓存容量: {cache_config.l1_config.max_capacity if cache_config.l1_config else 'None'}")
            
            # 添加带缓存的MongoDB数据库
            print("\n🔧 添加带缓存的MongoDB数据库...")
            response = bridge.add_mongodb_database(
                alias="test_cached_only",
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
                print(f"❌ 添加数据库失败: {result.get('error')}")
                return False
            
            print(f"✅ 数据库添加成功: {result.get('data')}")
            
            # 触发适配器创建
            print("\n🔍 触发适配器创建...")
            try:
                bridge.find_by_id("dummy_collection", "dummy_id", "test_cached_only")
            except:
                pass  # 忽略查询错误，只关注适配器创建日志
            
            print("✅ 场景1测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 场景1测试失败: {e}")
            return False
    
    def test_scenario_2_non_cache_only(self) -> bool:
        """测试场景2：单独非缓存模式"""
        print("\n🧪 测试场景2：单独非缓存模式")
        print("=" * 40)
        
        try:
            # 创建数据库队列桥接器
            bridge = create_db_queue_bridge()
            
            # 不创建缓存配置
            cache_config = None
            tls_config = self.create_tls_config()
            zstd_config = self.create_zstd_config(False)
            
            print(f"📋 缓存配置状态: {cache_config}")
            
            # 添加不带缓存的MongoDB数据库
            print("\n🔧 添加不带缓存的MongoDB数据库...")
            response = bridge.add_mongodb_database(
                alias="test_non_cached_only",
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
                print(f"❌ 添加数据库失败: {result.get('error')}")
                return False
            
            print(f"✅ 数据库添加成功: {result.get('data')}")
            
            # 触发适配器创建
            print("\n🔍 触发适配器创建...")
            try:
                bridge.find_by_id("dummy_collection", "dummy_id", "test_non_cached_only")
            except:
                pass  # 忽略查询错误，只关注适配器创建日志
            
            print("✅ 场景2测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 场景2测试失败: {e}")
            return False
    
    def test_scenario_3_mixed_mode(self) -> bool:
        """测试场景3：混合模式（缓存+非缓存）"""
        print("\n🧪 测试场景3：混合模式（缓存+非缓存）")
        print("=" * 40)
        
        try:
            # 创建数据库队列桥接器
            bridge = create_db_queue_bridge()
            
            # 创建缓存配置
            cache_config = self.create_cache_config()
            tls_config = self.create_tls_config()
            zstd_config = self.create_zstd_config(False)
            
            print(f"📋 缓存配置状态: enabled={cache_config.enabled}")
            
            # 添加带缓存的MongoDB数据库
            print("\n🔧 添加带缓存的MongoDB数据库...")
            response1 = bridge.add_mongodb_database(
                alias="test_mixed_cached",
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
            
            result1 = json.loads(response1)
            if not result1.get("success"):
                print(f"❌ 添加缓存数据库失败: {result1.get('error')}")
                return False
            
            print(f"✅ 缓存数据库添加成功: {result1.get('data')}")
            
            # 添加不带缓存的MongoDB数据库
            print("\n🔧 添加不带缓存的MongoDB数据库...")
            response2 = bridge.add_mongodb_database(
                alias="test_mixed_non_cached",
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
                cache_config=None,  # 不启用缓存
                tls_config=tls_config,
                zstd_config=zstd_config
            )
            
            result2 = json.loads(response2)
            if not result2.get("success"):
                print(f"❌ 添加非缓存数据库失败: {result2.get('error')}")
                return False
            
            print(f"✅ 非缓存数据库添加成功: {result2.get('data')}")
            
            # 触发两个数据库的适配器创建
            print("\n🔍 触发缓存数据库适配器创建...")
            try:
                bridge.find_by_id("dummy_collection", "dummy_id", "test_mixed_cached")
            except:
                pass  # 忽略查询错误，只关注适配器创建日志
            
            print("\n🔍 触发非缓存数据库适配器创建...")
            try:
                bridge.find_by_id("dummy_collection", "dummy_id", "test_mixed_non_cached")
            except:
                pass  # 忽略查询错误，只关注适配器创建日志
            
            print("✅ 场景3测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 场景3测试失败: {e}")
            return False
    
    def cleanup(self):
        """清理资源"""
        try:
            if os.path.exists(self.test_data_dir):
                import shutil
                shutil.rmtree(self.test_data_dir)
                print(f"🧹 清理测试数据目录: {self.test_data_dir}")
        except Exception as e:
            print(f"⚠️ 清理资源时出错: {e}")
    
    def run_all_tests(self) -> int:
        """运行所有测试场景"""
        print("🚀 RatQuickDB Python 缓存配置传递调试测试")
        print("============================================")
        
        try:
            print(f"📦 RatQuickDB 版本: {get_version()}")
        except Exception as e:
            print(f"⚠️ 无法获取版本信息: {e}")
        
        success_count = 0
        total_tests = 3
        
        try:
            # 创建测试数据目录
            os.makedirs(self.test_data_dir, exist_ok=True)
            
            # 运行测试场景1
            if self.test_scenario_1_cache_only():
                success_count += 1
            
            # 运行测试场景2
            if self.test_scenario_2_non_cache_only():
                success_count += 1
            
            # 运行测试场景3
            if self.test_scenario_3_mixed_mode():
                success_count += 1
            
            # 总结测试结果
            print("\n📊 测试结果总结")
            print("=" * 40)
            print(f"✅ 成功测试: {success_count}/{total_tests}")
            print(f"❌ 失败测试: {total_tests - success_count}/{total_tests}")
            
            if success_count == total_tests:
                print("🎯 所有测试场景都已完成！")
                print("\n🔍 请检查上述日志中的适配器类型信息：")
                print("   - 缓存模式应该显示：'缓存适配器'")
                print("   - 非缓存模式应该显示：'普通适配器'")
                print("   - 如果都显示'普通适配器'，则说明缓存配置传递存在问题")
                return 0
            else:
                print("❌ 部分测试失败，请检查错误信息")
                return 1
            
        except Exception as e:
            print(f"❌ 测试执行失败: {e}")
            return 1
        finally:
            self.cleanup()


def main() -> int:
    """主函数"""
    test = CacheConfigDebugTest()
    
    try:
        return test.run_all_tests()
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