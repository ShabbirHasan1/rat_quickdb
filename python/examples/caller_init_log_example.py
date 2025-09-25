#!/usr/bin/env python3
"""
调用者初始化日志示例

这个示例展示了如何作为调用者来正确初始化日志系统
然后使用rat_quickdb进行数据库操作
"""

import asyncio
import sys
import os

# 添加当前目录到Python路径，以便导入rat_quickdb_py
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
    from rat_quickdb_py import (
        init_logging_advanced, init_logging, init_logging_with_level,
        is_logging_initialized, log_info, log_error, log_warn, log_debug, log_trace,
        DbQueueBridge, create_db_queue_bridge,
        PyCacheConfig, PyTtlConfig, PyCompressionConfig,
        FieldType, string_field, integer_field, boolean_field,
        register_model, FieldDefinition, IndexDefinition, ModelMeta
    )
except ImportError as e:
    print(f"导入错误: {e}")
    print("请确保已运行 'maturin develop' 来构建Python绑定")
    sys.exit(1)


# 全局变量跟踪日志初始化状态
_logging_initialized = False

def demo_basic_logging():
    """演示基本的日志初始化"""
    global _logging_initialized
    print("=== 基本日志初始化演示 ===")

    # 检查日志系统状态
    print(f"日志系统初始化状态: {is_logging_initialized()}")

    # 只有未初始化时才进行初始化
    if not _logging_initialized:
        print("初始化基本日志系统...")
        init_logging()
        _logging_initialized = True
    else:
        print("日志系统已经初始化，跳过重复初始化")

    # 测试日志输出
    log_info("这是一条信息日志")
    log_warn("这是一条警告日志")
    log_error("这是一条错误日志")
    log_debug("这是一条调试日志")
    log_trace("这是一条跟踪日志")

    print("✅ 基本日志初始化完成\n")


def demo_advanced_logging():
    """演示高级日志配置"""
    global _logging_initialized
    print("=== 高级日志配置演示 ===")

    # 只有未初始化时才进行初始化
    if not _logging_initialized:
        # 使用高级日志配置
        print("初始化高级日志系统...")
        init_logging_advanced(
            level="debug",  # 设置调试级别
            enable_color=True,  # 启用颜色
            timestamp_format="%Y-%m-%d %H:%M:%S",  # 自定义时间格式
            custom_format_template="[{timestamp}] {level} PYTHON - {message}"  # 自定义格式
        )
        _logging_initialized = True
    else:
        print("日志系统已经初始化，跳过重复初始化")

    # 测试不同级别的日志输出
    log_info("使用高级配置的信息日志")
    log_warn("使用高级配置的警告日志")
    log_error("使用高级配置的错误日志")
    log_debug("使用高级配置的调试日志")
    log_trace("使用高级配置的跟踪日志")

    print("✅ 高级日志配置完成\n")


def demo_level_control():
    """演示日志级别控制"""
    global _logging_initialized
    print("=== 日志级别控制演示 ===")

    # 注意：由于rat_logger的限制，一旦初始化就无法重新配置
    # 这个演示主要是为了展示API的使用方式
    if not _logging_initialized:
        # 初始化为错误级别
        print("初始化为错误级别...")
        init_logging_with_level("error")
        _logging_initialized = True
    else:
        print("日志系统已经初始化，无法重新配置级别")

    print("根据当前配置显示日志:")
    log_trace("这条跟踪日志可能不会显示")
    log_debug("这条调试日志可能不会显示")
    log_info("这条信息日志可能不会显示")
    log_warn("这条警告日志可能不会显示")
    log_error("这条错误日志会显示")

    print("✅ 日志级别控制演示完成\n")


async def demo_database_operations():
    """演示结合数据库操作的日志使用"""
    global _logging_initialized
    print("=== 数据库操作日志演示 ===")

    try:
        # 只有未初始化时才进行初始化
        if not _logging_initialized:
            init_logging_with_level("info")
            _logging_initialized = True
        else:
            print("日志系统已经初始化，跳过重复初始化")

        log_info("开始数据库操作演示")

        # 创建内存SQLite数据库
        log_info("创建数据库连接...")
        # 注意：这里使用正确的API，但create_db_queue_bridge的API需要进一步确认
        # bridge = create_db_queue_bridge()
        log_info("数据库连接创建成功（跳过实际创建，只演示日志功能）")

        log_info("数据库操作演示完成")

    except Exception as e:
        log_error(f"数据库操作失败: {e}")
        print(f"错误: {e}")


def main():
    """主函数"""
    print("🚀 RAT QuickDB Python绑定 - 调用者初始化日志示例")
    print("=" * 60)

    # 演示不同的日志初始化方式
    demo_basic_logging()

    demo_advanced_logging()

    demo_level_control()

    # 演示数据库操作中的日志使用
    asyncio.run(demo_database_operations())

    print("=" * 60)
    print("📋 总结:")
    print("1. 调用者完全控制日志系统的初始化")
    print("2. 提供了多种日志配置选项:")
    print("   - init_logging(): 基本配置")
    print("   - init_logging_with_level(): 指定级别")
    print("   - init_logging_advanced(): 完全自定义配置")
    print("3. 日志系统完全可选，调用者可以自行实现")
    print("4. 支持所有标准的日志级别: trace, debug, info, warn, error")
    print("5. 提供了日志状态检查功能")


if __name__ == "__main__":
    main()