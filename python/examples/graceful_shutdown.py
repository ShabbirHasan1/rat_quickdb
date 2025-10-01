#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
优雅关闭机制工具类

为 rat_quickdb Python 示例提供统一的资源管理和优雅关闭功能。
支持信号处理、资源清理、后台任务停止等功能。

使用方法:
1. 继承 GracefulShutdownMixin 类
2. 实现 cleanup_resources() 方法
3. 在 main 函数中使用 with_graceful_shutdown 装饰器
"""

import os
import sys
import signal
import atexit
import threading
import time
import shutil
import json
from typing import Optional, Callable, List, Dict, Any
from contextlib import contextmanager
from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime, timezone


@dataclass
class ShutdownConfig:
    """优雅关闭配置"""
    # 关闭超时时间（秒）
    shutdown_timeout: int = 30
    # 是否启用信号处理
    enable_signal_handling: bool = True
    # 是否在退出时自动清理
    auto_cleanup_on_exit: bool = True
    # 是否显示详细的关闭日志
    verbose_logging: bool = True
    # 强制关闭前的警告时间（秒）
    force_shutdown_warning: int = 5


class GracefulShutdownMixin(ABC):
    """优雅关闭混入类
    
    提供统一的资源管理和优雅关闭功能。
    子类需要实现 cleanup_resources() 方法。
    """
    
    def __init__(self, shutdown_config: Optional[ShutdownConfig] = None):
        self.shutdown_config = shutdown_config or ShutdownConfig()
        self._shutdown_requested = threading.Event()
        self._cleanup_completed = threading.Event()
        self._resources_to_cleanup: List[Callable[[], None]] = []
        self._background_tasks: List[threading.Thread] = []
        self._temp_files: List[str] = []
        self._temp_dirs: List[str] = []
        self._database_connections: List[Any] = []
        self._shutdown_lock = threading.Lock()
        self._is_shutting_down = False
        
        # 注册信号处理器
        if self.shutdown_config.enable_signal_handling:
            self._register_signal_handlers()
        
        # 注册退出处理器
        if self.shutdown_config.auto_cleanup_on_exit:
            atexit.register(self._atexit_handler)
    
    def _register_signal_handlers(self):
        """注册信号处理器"""
        def signal_handler(signum, frame):
            signal_name = signal.Signals(signum).name
            if self.shutdown_config.verbose_logging:
                print(f"\n🛑 收到信号 {signal_name}，开始优雅关闭...")
            self.request_shutdown()
        
        # 注册常见的终止信号
        try:
            signal.signal(signal.SIGINT, signal_handler)   # Ctrl+C
            signal.signal(signal.SIGTERM, signal_handler)  # 终止信号
            if hasattr(signal, 'SIGHUP'):
                signal.signal(signal.SIGHUP, signal_handler)   # 挂起信号
        except (OSError, ValueError) as e:
            if self.shutdown_config.verbose_logging:
                print(f"⚠️  注册信号处理器失败: {e}")
    
    def _atexit_handler(self):
        """退出处理器"""
        if not self._cleanup_completed.is_set():
            if self.shutdown_config.verbose_logging:
                print("\n🧹 程序退出时自动清理资源...")
            self.shutdown()
    
    def request_shutdown(self):
        """请求优雅关闭"""
        with self._shutdown_lock:
            if self._is_shutting_down:
                return
            self._is_shutting_down = True
            self._shutdown_requested.set()
    
    def is_shutdown_requested(self) -> bool:
        """检查是否已请求关闭"""
        return self._shutdown_requested.is_set()
    
    def add_cleanup_resource(self, cleanup_func: Callable[[], None]):
        """添加需要清理的资源"""
        self._resources_to_cleanup.append(cleanup_func)
    
    def add_background_task(self, task: threading.Thread):
        """添加后台任务"""
        self._background_tasks.append(task)
    
    def add_temp_file(self, file_path: str):
        """添加临时文件"""
        self._temp_files.append(file_path)
    
    def add_temp_dir(self, dir_path: str):
        """添加临时目录"""
        self._temp_dirs.append(dir_path)
    
    def add_database_connection(self, connection: Any):
        """添加数据库连接"""
        self._database_connections.append(connection)
    
    def wait_for_shutdown(self, timeout: Optional[float] = None) -> bool:
        """等待关闭信号
        
        Args:
            timeout: 等待超时时间（秒），None表示无限等待
            
        Returns:
            bool: 是否收到关闭信号
        """
        return self._shutdown_requested.wait(timeout)
    
    def shutdown(self) -> bool:
        """执行优雅关闭
        
        Returns:
            bool: 是否成功关闭
        """
        with self._shutdown_lock:
            if self._cleanup_completed.is_set():
                return True
            
            if self.shutdown_config.verbose_logging:
                print("\n🛑 开始优雅关闭流程...")
            
            start_time = time.time()
            success = True
            
            try:
                # 1. 停止后台任务
                success &= self._stop_background_tasks()
                
                # 2. 关闭数据库连接
                success &= self._close_database_connections()
                
                # 3. 执行自定义资源清理
                success &= self._execute_custom_cleanup()
                
                # 4. 清理临时文件和目录
                success &= self._cleanup_temp_resources()
                
                # 5. 执行额外的清理函数
                success &= self._execute_cleanup_functions()
                
                elapsed_time = time.time() - start_time
                
                if self.shutdown_config.verbose_logging:
                    status = "✅ 成功" if success else "⚠️  部分失败"
                    print(f"🏁 优雅关闭完成 ({status}) - 耗时: {elapsed_time:.2f}秒")
                
                self._cleanup_completed.set()
                return success
                
            except Exception as e:
                if self.shutdown_config.verbose_logging:
                    print(f"❌ 优雅关闭过程中发生错误: {e}")
                    import traceback
                    traceback.print_exc()
                return False
    
    def _stop_background_tasks(self) -> bool:
        """停止后台任务"""
        if not self._background_tasks:
            return True
        
        if self.shutdown_config.verbose_logging:
            print(f"🔄 停止 {len(self._background_tasks)} 个后台任务...")
        
        success = True
        timeout = self.shutdown_config.shutdown_timeout
        
        for i, task in enumerate(self._background_tasks):
            try:
                if task.is_alive():
                    if self.shutdown_config.verbose_logging:
                        print(f"  停止后台任务 {i+1}/{len(self._background_tasks)}...")
                    
                    # 等待任务自然结束
                    task.join(timeout=timeout)
                    
                    if task.is_alive():
                        if self.shutdown_config.verbose_logging:
                            print(f"  ⚠️  后台任务 {i+1} 未在 {timeout} 秒内结束")
                        success = False
                    else:
                        if self.shutdown_config.verbose_logging:
                            print(f"  ✅ 后台任务 {i+1} 已停止")
                            
            except Exception as e:
                if self.shutdown_config.verbose_logging:
                    print(f"  ❌ 停止后台任务 {i+1} 失败: {e}")
                success = False
        
        return success
    
    def _close_database_connections(self) -> bool:
        """关闭数据库连接"""
        if not self._database_connections:
            return True
        
        if self.shutdown_config.verbose_logging:
            print(f"🔌 关闭 {len(self._database_connections)} 个数据库连接...")
        
        success = True
        
        for i, connection in enumerate(self._database_connections):
            try:
                if hasattr(connection, 'close'):
                    connection.close()
                elif hasattr(connection, 'shutdown'):
                    connection.shutdown()
                elif hasattr(connection, 'disconnect'):
                    connection.disconnect()
                
                if self.shutdown_config.verbose_logging:
                    print(f"  ✅ 数据库连接 {i+1} 已关闭")
                    
            except Exception as e:
                if self.shutdown_config.verbose_logging:
                    print(f"  ❌ 关闭数据库连接 {i+1} 失败: {e}")
                success = False
        
        return success
    
    def _execute_custom_cleanup(self) -> bool:
        """执行自定义资源清理"""
        try:
            if self.shutdown_config.verbose_logging:
                print("🧹 执行自定义资源清理...")
            
            self.cleanup_resources()
            
            if self.shutdown_config.verbose_logging:
                print("  ✅ 自定义资源清理完成")
            return True
            
        except Exception as e:
            if self.shutdown_config.verbose_logging:
                print(f"  ❌ 自定义资源清理失败: {e}")
                import traceback
                traceback.print_exc()
            return False
    
    def _cleanup_temp_resources(self) -> bool:
        """清理临时文件和目录"""
        success = True
        
        # 清理临时文件
        if self._temp_files:
            if self.shutdown_config.verbose_logging:
                print(f"🗑️  清理 {len(self._temp_files)} 个临时文件...")
            
            for file_path in self._temp_files:
                try:
                    if os.path.exists(file_path):
                        os.remove(file_path)
                        if self.shutdown_config.verbose_logging:
                            print(f"  ✅ 已删除临时文件: {file_path}")
                except Exception as e:
                    if self.shutdown_config.verbose_logging:
                        print(f"  ❌ 删除临时文件失败 {file_path}: {e}")
                    success = False
        
        # 清理临时目录
        if self._temp_dirs:
            if self.shutdown_config.verbose_logging:
                print(f"🗑️  清理 {len(self._temp_dirs)} 个临时目录...")
            
            for dir_path in self._temp_dirs:
                try:
                    if os.path.exists(dir_path):
                        shutil.rmtree(dir_path)
                        if self.shutdown_config.verbose_logging:
                            print(f"  ✅ 已删除临时目录: {dir_path}")
                except Exception as e:
                    if self.shutdown_config.verbose_logging:
                        print(f"  ❌ 删除临时目录失败 {dir_path}: {e}")
                    success = False
        
        return success
    
    def _execute_cleanup_functions(self) -> bool:
        """执行额外的清理函数"""
        if not self._resources_to_cleanup:
            return True
        
        if self.shutdown_config.verbose_logging:
            print(f"🔧 执行 {len(self._resources_to_cleanup)} 个清理函数...")
        
        success = True
        
        for i, cleanup_func in enumerate(self._resources_to_cleanup):
            try:
                cleanup_func()
                if self.shutdown_config.verbose_logging:
                    print(f"  ✅ 清理函数 {i+1} 执行完成")
            except Exception as e:
                if self.shutdown_config.verbose_logging:
                    print(f"  ❌ 清理函数 {i+1} 执行失败: {e}")
                success = False
        
        return success
    
    @abstractmethod
    def cleanup_resources(self):
        """子类需要实现的资源清理方法"""
        pass


def with_graceful_shutdown(shutdown_config: Optional[ShutdownConfig] = None):
    """优雅关闭装饰器
    
    用于装饰 main 函数，提供统一的异常处理和资源清理。
    
    Args:
        shutdown_config: 关闭配置
    """
    def decorator(func):
        def wrapper(*args, **kwargs):
            config = shutdown_config or ShutdownConfig()
            start_time = datetime.now()
            
            if config.verbose_logging:
                print(f"🚀 程序启动: {func.__name__} - {start_time.strftime('%Y-%m-%d %H:%M:%S')}")
            
            try:
                # 执行主函数
                result = func(*args, **kwargs)
                
                if config.verbose_logging:
                    end_time = datetime.now()
                    duration = (end_time - start_time).total_seconds()
                    print(f"🎯 程序正常结束 - 运行时间: {duration:.2f}秒")
                
                return result
                
            except KeyboardInterrupt:
                if config.verbose_logging:
                    print("\n⚠️  程序被用户中断 (Ctrl+C)")
                return 1
                
            except Exception as e:
                if config.verbose_logging:
                    print(f"\n❌ 程序执行失败: {e}")
                    import traceback
                    traceback.print_exc()
                return 1
                
            finally:
                if config.verbose_logging:
                    end_time = datetime.now()
                    duration = (end_time - start_time).total_seconds()
                    print(f"\n📊 程序执行统计:")
                    print(f"   • 开始时间: {start_time.strftime('%Y-%m-%d %H:%M:%S')}")
                    print(f"   • 结束时间: {end_time.strftime('%Y-%m-%d %H:%M:%S')}")
                    print(f"   • 运行时长: {duration:.2f}秒")
        
        return wrapper
    return decorator


@contextmanager
def graceful_shutdown_context(shutdown_config: Optional[ShutdownConfig] = None):
    """优雅关闭上下文管理器
    
    用于 with 语句，确保资源在退出时被正确清理。
    
    Args:
        shutdown_config: 关闭配置
    """
    config = shutdown_config or ShutdownConfig()
    cleanup_functions = []
    
    class ContextManager:
        def add_cleanup(self, func: Callable[[], None]):
            cleanup_functions.append(func)
        
        def add_temp_file(self, file_path: str):
            def cleanup():
                try:
                    if os.path.exists(file_path):
                        os.remove(file_path)
                        if config.verbose_logging:
                            print(f"🗑️  已删除临时文件: {file_path}")
                except Exception as e:
                    if config.verbose_logging:
                        print(f"❌ 删除临时文件失败 {file_path}: {e}")
            cleanup_functions.append(cleanup)
        
        def add_temp_dir(self, dir_path: str):
            def cleanup():
                try:
                    if os.path.exists(dir_path):
                        shutil.rmtree(dir_path)
                        if config.verbose_logging:
                            print(f"🗑️  已删除临时目录: {dir_path}")
                except Exception as e:
                    if config.verbose_logging:
                        print(f"❌ 删除临时目录失败 {dir_path}: {e}")
            cleanup_functions.append(cleanup)
    
    context = ContextManager()
    
    try:
        yield context
    finally:
        if config.verbose_logging and cleanup_functions:
            print(f"\n🧹 执行 {len(cleanup_functions)} 个清理操作...")
        
        for i, cleanup_func in enumerate(cleanup_functions):
            try:
                cleanup_func()
                if config.verbose_logging:
                    print(f"  ✅ 清理操作 {i+1} 完成")
            except Exception as e:
                if config.verbose_logging:
                    print(f"  ❌ 清理操作 {i+1} 失败: {e}")


class ResourceTracker:
    """资源跟踪器
    
    用于跟踪和管理程序中使用的各种资源。
    """
    
    def __init__(self):
        self._resources: Dict[str, List[Any]] = {
            'files': [],
            'directories': [],
            'connections': [],
            'threads': [],
            'processes': [],
            'custom': []
        }
        self._lock = threading.Lock()
    
    def add_file(self, file_path: str):
        """添加文件资源"""
        with self._lock:
            self._resources['files'].append(file_path)
    
    def add_directory(self, dir_path: str):
        """添加目录资源"""
        with self._lock:
            self._resources['directories'].append(dir_path)
    
    def add_connection(self, connection: Any):
        """添加连接资源"""
        with self._lock:
            self._resources['connections'].append(connection)
    
    def add_thread(self, thread: threading.Thread):
        """添加线程资源"""
        with self._lock:
            self._resources['threads'].append(thread)
    
    def add_custom_resource(self, resource: Any, cleanup_func: Callable[[Any], None]):
        """添加自定义资源"""
        with self._lock:
            self._resources['custom'].append((resource, cleanup_func))
    
    def get_resource_summary(self) -> Dict[str, int]:
        """获取资源摘要"""
        with self._lock:
            return {
                resource_type: len(resources)
                for resource_type, resources in self._resources.items()
            }
    
    def cleanup_all(self, verbose: bool = True) -> bool:
        """清理所有资源"""
        success = True
        
        with self._lock:
            # 清理文件
            for file_path in self._resources['files']:
                try:
                    if os.path.exists(file_path):
                        os.remove(file_path)
                        if verbose:
                            print(f"🗑️  已删除文件: {file_path}")
                except Exception as e:
                    if verbose:
                        print(f"❌ 删除文件失败 {file_path}: {e}")
                    success = False
            
            # 清理目录
            for dir_path in self._resources['directories']:
                try:
                    if os.path.exists(dir_path):
                        shutil.rmtree(dir_path)
                        if verbose:
                            print(f"🗑️  已删除目录: {dir_path}")
                except Exception as e:
                    if verbose:
                        print(f"❌ 删除目录失败 {dir_path}: {e}")
                    success = False
            
            # 关闭连接
            for connection in self._resources['connections']:
                try:
                    if hasattr(connection, 'close'):
                        connection.close()
                    elif hasattr(connection, 'shutdown'):
                        connection.shutdown()
                    if verbose:
                        print(f"🔌 已关闭连接: {type(connection).__name__}")
                except Exception as e:
                    if verbose:
                        print(f"❌ 关闭连接失败: {e}")
                    success = False
            
            # 停止线程
            for thread in self._resources['threads']:
                try:
                    if thread.is_alive():
                        thread.join(timeout=5)
                        if thread.is_alive():
                            if verbose:
                                print(f"⚠️  线程 {thread.name} 未在5秒内结束")
                            success = False
                        else:
                            if verbose:
                                print(f"🔄 已停止线程: {thread.name}")
                except Exception as e:
                    if verbose:
                        print(f"❌ 停止线程失败: {e}")
                    success = False
            
            # 清理自定义资源
            for resource, cleanup_func in self._resources['custom']:
                try:
                    cleanup_func(resource)
                    if verbose:
                        print(f"🔧 已清理自定义资源: {type(resource).__name__}")
                except Exception as e:
                    if verbose:
                        print(f"❌ 清理自定义资源失败: {e}")
                    success = False
        
        return success


# 全局资源跟踪器实例
_global_resource_tracker = ResourceTracker()


def get_global_resource_tracker() -> ResourceTracker:
    """获取全局资源跟踪器"""
    return _global_resource_tracker


if __name__ == "__main__":
    # 演示用法
    
    class DemoApp(GracefulShutdownMixin):
        def __init__(self):
            super().__init__(ShutdownConfig(
                shutdown_timeout=10,
                verbose_logging=True
            ))
            
            # 模拟一些资源
            self.add_temp_file("/tmp/demo_file.txt")
            self.add_temp_dir("/tmp/demo_dir")
            
            # 创建测试文件和目录
            with open("/tmp/demo_file.txt", "w") as f:
                f.write("demo content")
            os.makedirs("/tmp/demo_dir", exist_ok=True)
        
        def cleanup_resources(self):
            print("  🧹 执行应用特定的资源清理...")
            # 这里可以添加应用特定的清理逻辑
            pass
        
        def run(self):
            print("🚀 应用开始运行...")
            
            # 模拟运行一段时间
            for i in range(10):
                if self.is_shutdown_requested():
                    print("收到关闭请求，提前退出循环")
                    break
                print(f"运行中... {i+1}/10")
                time.sleep(1)
            
            print("✅ 应用运行完成")
    
    @with_graceful_shutdown(ShutdownConfig(verbose_logging=True))
    def main():
        app = DemoApp()
        try:
            app.run()
            return 0
        finally:
            app.shutdown()
    
    exit(main())