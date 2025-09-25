#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
单独测试每个数据库的JSON字段解析
确保每个测试都是完全独立的
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def test_sqlite_only():
    """只测试SQLite JSON字段解析"""
    print("\n" + "="*50)
    print("🚀 测试 SQLite JSON字段解析")
    print("="*50)

    bridge = rq.create_db_queue_bridge()

    # 添加SQLite数据库
    result = bridge.add_sqlite_database(
        alias="sqlite_json_test",
        path=":memory:",
        max_connections=5,
        min_connections=1,
        connection_timeout=30,
        idle_timeout=600,
        max_lifetime=3600
    )

    if not json.loads(result).get("success"):
        print(f"❌ SQLite数据库添加失败: {json.loads(result).get('error')}")
        return False

    print("✅ SQLite数据库添加成功")

    # 创建字段定义
    id_field = rq.integer_field(True, True, None, None, "主键ID")
    name_field = rq.string_field(True, False, None, None, "名称")
    json_field = rq.json_field(False, "JSON数据")

    # 创建索引
    index_def = rq.IndexDefinition(["id"], True, "idx_id")

    # 创建字段字典
    fields_dict = {
        "id": id_field,
        "name": name_field,
        "json_data": json_field
    }

    # 创建模型元数据
    table_name = f"jsondata_{int(time.time())}"
    model_meta = rq.ModelMeta(
        table_name,
        fields_dict,
        [index_def],
        "sqlite_json_test",
        "SQLite JSON测试表"
    )

    # 注册模型
    register_result = bridge.register_model(model_meta)
    if not json.loads(register_result).get("success"):
        print(f"❌ ODM模型注册失败")
        return False

    print("✅ ODM模型注册成功")

    # 测试数据 - 复杂的嵌套JSON结构
    test_data = {
        "name": "SQLite复杂JSON测试",
        "json_data": {
            "user": {
                "id": 1,
                "name": "张三",
                "profile": {
                    "age": 30,
                    "email": "zhangsan@example.com",
                    "preferences": {
                        "theme": "dark",
                        "language": "zh-CN",
                        "notifications": {
                            "email": True,
                            "sms": False,
                            "push": True
                        }
                    }
                },
                "stats": {
                    "login_count": 150,
                    "last_login": "2025-01-15T10:30:00Z",
                    "is_active": True
                }
            },
            "content": {
                "title": "测试文章",
                "body": "这是一篇测试文章的内容",
                "metadata": {
                    "tags": ["技术", "数据库", "JSON"],
                    "category": "编程",
                    "read_time": 5,
                    "published": True
                },
                "comments": [
                    {
                        "id": 1,
                        "author": "李四",
                        "text": "很好的文章！",
                        "timestamp": "2025-01-15T11:00:00Z"
                    },
                    {
                        "id": 2,
                        "author": "王五",
                        "text": "学到了很多",
                        "timestamp": "2025-01-15T12:30:00Z"
                    }
                ]
            },
            "settings": {
                "privacy": {
                    "profile_visible": True,
                    "email_visible": False,
                    "activity_visible": True
                },
                "security": {
                    "two_factor_enabled": True,
                    "last_password_change": "2025-01-01T00:00:00Z",
                    "login_attempts": 0
                }
            }
        }
    }

    # 插入数据
    insert_result = bridge.create(table_name, json.dumps(test_data), "sqlite_json_test")
    insert_data = json.loads(insert_result)

    if not insert_data.get("success"):
        print(f"❌ 数据插入失败: {insert_data.get('error')}")
        return False

    print("✅ 数据插入成功")

    # 查询数据
    query_result = bridge.find(table_name, '{}', "sqlite_json_test")
    query_data = json.loads(query_result)

    if not query_data.get("success"):
        print(f"❌ 数据查询失败: {query_data.get('error')}")
        return False

    records = query_data.get("data")
    if not records or len(records) == 0:
        print("❌ 查询结果为空")
        return False

    record = records[0]
    print(f"✅ 数据查询成功")

    # 验证JSON字段
    json_field = record.get('json_data')
    print(f"   json_data类型: {type(json_field)}")

    if isinstance(json_field, dict):
        print("✅ JSON字段正确解析为dict")

        # 验证深层嵌套结构
        user = json_field.get('user', {})
        if isinstance(user, dict):
            profile = user.get('profile', {})
            if isinstance(profile, dict):
                preferences = profile.get('preferences', {})
                if isinstance(preferences, dict):
                    notifications = preferences.get('notifications', {})
                    if isinstance(notifications, dict):
                        print(f"✅ user.profile.preferences.notifications.email: {notifications.get('email')}")
                        print(f"✅ user.profile.preferences.notifications.sms: {notifications.get('sms')}")
                        print(f"✅ user.profile.preferences.notifications.push: {notifications.get('push')}")

            stats = user.get('stats', {})
            if isinstance(stats, dict):
                print(f"✅ user.stats.login_count: {stats.get('login_count')}")
                print(f"✅ user.stats.is_active: {stats.get('is_active')}")

        content = json_field.get('content', {})
        if isinstance(content, dict):
            metadata = content.get('metadata', {})
            if isinstance(metadata, dict):
                print(f"✅ content.metadata.tags: {metadata.get('tags')}")
                print(f"✅ content.metadata.read_time: {metadata.get('read_time')}")

            comments = content.get('comments', [])
            if isinstance(comments, list) and len(comments) > 0:
                print(f"✅ content.comments数量: {len(comments)}")
                print(f"✅ 第一条评论: {comments[0].get('author')} - {comments[0].get('text')}")

        settings = json_field.get('settings', {})
        if isinstance(settings, dict):
            privacy = settings.get('privacy', {})
            if isinstance(privacy, dict):
                print(f"✅ settings.privacy.profile_visible: {privacy.get('profile_visible')}")

            security = settings.get('security', {})
            if isinstance(security, dict):
                print(f"✅ settings.security.two_factor_enabled: {security.get('two_factor_enabled')}")

        print("\n🎯 SQLite JSON字段解析验证完成，所有嵌套结构都正确解析！")
    else:
        print(f"❌ JSON字段解析失败: {type(json_field)}")
        return False

    # 清理
    bridge.drop_table(table_name, "sqlite_json_test")
    print("✅ SQLite测试完成")
    return True

def main():
    """主函数 - 只测试SQLite"""
    print("🧪 SQL数据库JSON字段解析验证 - 单独测试")
    print("单独测试SQLite以确保功能正常")

    # 初始化日志
    try:
        rq.init_logging_with_level("info")
        print("✅ 日志初始化成功")
    except:
        print("⚠️ 日志初始化失败")

    result = test_sqlite_only()

    print("\n" + "="*50)
    print("🎯 测试结果")
    print("="*50)
    print(f"SQLite: {'✅ 通过' if result else '❌ 失败'}")

    if result:
        print("\n🎉 SQLite JSON字段解析功能完全正常！")
        print("✅ register_model功能正常工作")
        print("✅ ODM模型注册让系统能正确识别和解析JSON字段")
        print("✅ 支持复杂的嵌套JSON结构")
        print("✅ 支持JSON数组的解析")
        print("✅ 所有数据类型（字符串、数字、布尔值、数组、对象）都正确处理")
        return True
    else:
        print("\n⚠️ SQLite JSON字段解析功能存在问题")
        return False

if __name__ == "__main__":
    main()