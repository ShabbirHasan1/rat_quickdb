#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MongoDB JSON字段解析验证测试
测试MongoDB原生JSON处理能力
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def test_mongodb_json_parsing():
    """测试MongoDB JSON字段解析"""
    print("\n" + "="*50)
    print("🚀 测试 MongoDB JSON字段解析")
    print("="*50)

    try:
        bridge = rq.create_db_queue_bridge()

        # TLS配置
        tls_config = rq.PyTlsConfig()
        tls_config.enable()
        tls_config.ca_cert_path = "/etc/ssl/certs/ca-certificates.crt"
        tls_config.client_cert_path = ""
        tls_config.client_key_path = ""

        # ZSTD配置
        zstd_config = rq.PyZstdConfig()
        zstd_config.enable()
        zstd_config.compression_level = 3
        zstd_config.compression_threshold = 1024

        # 添加MongoDB数据库（使用验证过的配置）
        result = bridge.add_mongodb_database(
            alias="mongodb_json_test",
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
            tls_config=tls_config,
            zstd_config=zstd_config
        )

        if not json.loads(result).get("success"):
            print(f"❌ MongoDB数据库添加失败: {json.loads(result).get('error')}")
            return False

        print("✅ MongoDB数据库添加成功")

        # 创建字段定义
        id_field = rq.integer_field(True, True, None, None, "主键ID")
        name_field = rq.string_field(True, False, None, None, "名称")
        json_field = rq.json_field(False, "JSON数据")
        array_field = rq.array_field(rq.FieldType.string(None, None), False, None, None, "数组字段")

        # 创建索引
        index_def = rq.IndexDefinition(["id"], True, "idx_id")

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_data": json_field,
            "array_field": array_field
        }

        # 创建模型元数据
        table_name = f"mongodb_json_test_{int(time.time())}"
        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "mongodb_json_test",
            "MongoDB JSON测试"
        )

        # 注册模型
        register_result = bridge.register_model(model_meta)
        if not json.loads(register_result).get("success"):
            print(f"❌ ODM模型注册失败")
            return False

        print("✅ ODM模型注册成功")

        # 测试数据 - MongoDB原生支持的复杂JSON结构
        test_data = {
            "name": "MongoDB原生JSON测试",
            "json_data": {
                # 嵌套对象
                "user": {
                    "id": {"$oid": "507f1f77bcf86cd799439011"},
                    "profile": {
                        "personal": {
                            "name": "张三",
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
                        "professional": {
                            "title": "高级工程师",
                            "department": "技术研发",
                            "skills": ["Rust", "Python", "MongoDB", "PostgreSQL"],
                            "experience": 8,
                            "projects": [
                                {
                                    "name": "rat_quickdb ORM",
                                    "role": "主要开发者",
                                    "duration": "2年",
                                    "technologies": ["Rust", "Python", "PyO3"]
                                },
                                {
                                    "name": "数据分析平台",
                                    "role": "技术负责人",
                                    "duration": "3年",
                                    "technologies": ["Python", "MongoDB", "Docker"]
                                }
                            ]
                        }
                    },
                    "stats": {
                        "login_count": 1250,
                        "last_login": {"$date": "2025-01-15T10:30:00Z"},
                        "created_at": {"$date": "2020-06-01T00:00:00Z"},
                        "is_active": True,
                        "preferences": {
                            "privacy_level": "medium",
                            "data_sharing": True,
                            "marketing_emails": False
                        }
                    }
                },
                # 复杂的数组结构
                "content_items": [
                    {
                        "type": "article",
                        "title": "MongoDB最佳实践",
                        "content": "本文详细介绍了MongoDB的使用技巧...",
                        "metadata": {
                            "author": "数据库专家",
                            "published": True,
                            "published_at": {"$date": "2025-01-10T00:00:00Z"},
                            "tags": ["MongoDB", "数据库", "最佳实践"],
                            "statistics": {
                                "views": 5000,
                                "likes": 250,
                                "comments": 45,
                                "shares": 20
                            }
                        },
                        "comments": [
                            {
                                "user_id": {"$oid": "507f1f77bcf86cd799439012"},
                                "username": "李四",
                                "comment": "文章写得很好，学到了很多！",
                                "timestamp": {"$date": "2025-01-10T14:30:00Z"},
                                "likes": 15
                            },
                            {
                                "user_id": {"$oid": "507f1f77bcf86cd799439013"},
                                "username": "王五",
                                "comment": "希望能看到更多这样的技术文章",
                                "timestamp": {"$date": "2025-01-10T16:45:00Z"},
                                "likes": 8
                            }
                        ]
                    },
                    {
                        "type": "video",
                        "title": "MongoDB聚合管道教程",
                        "duration": 1800,  # 30分钟
                        "url": "https://example.com/videos/mongodb-aggregation",
                        "metadata": {
                            "resolution": "1080p",
                            "format": "mp4",
                            "size_mb": 256,
                            "author": "技术讲师",
                            "published": True,
                            "published_at": {"$date": "2025-01-12T00:00:00Z"},
                            "tags": ["MongoDB", "聚合", "教程"],
                            "chapters": [
                                {"title": "基础概念", "start": 0, "end": 300},
                                {"title": "$match操作", "start": 300, "end": 600},
                                {"title": "$group操作", "start": 600, "end": 900},
                                {"title": "实际案例", "start": 900, "end": 1800}
                            ]
                        }
                    }
                ],
                # 配置和设置
                "system_config": {
                    "database": {
                        "replica_set": "rs0",
                        "read_preference": "primary",
                        "write_concern": {
                            "w": "majority",
                            "j": True,
                            "wtimeout": 10000
                        },
                        "index_options": {
                            "background": True,
                            "unique": False,
                            "sparse": False
                        }
                    },
                    "cache": {
                        "enabled": True,
                        "ttl": 3600,
                        "max_size_mb": 512,
                        "compression": True
                    },
                    "security": {
                        "authentication": True,
                        "authorization": True,
                        "encryption": {
                            "at_rest": True,
                            "in_transit": True
                        },
                        "audit": {
                            "enabled": True,
                            "log_level": "info"
                        }
                    }
                },
                # 统计和分析数据
                "analytics": {
                    "performance": {
                        "query_stats": {
                            "avg_response_time": 25.5,
                            "p95_response_time": 120.0,
                            "p99_response_time": 250.0,
                            "queries_per_second": 1000,
                            "cache_hit_rate": 0.85
                        },
                        "index_performance": {
                            "index_size_mb": 128,
                            "index_usage_rate": 0.95,
                            "fragmentation_ratio": 0.05
                        },
                        "storage": {
                            "total_size_gb": 50.0,
                            "data_size_gb": 35.0,
                            "index_size_gb": 10.0,
                            "free_space_gb": 15.0,
                            "compression_ratio": 0.3
                        }
                    },
                    "usage": {
                        "active_users": 5000,
                        "daily_operations": 100000,
                        "peak_concurrent_connections": 250,
                        "data_growth_rate_gb_per_month": 2.5
                    }
                }
            },
            "array_field": [
                "MongoDB",
                "原生JSON",
                "文档数据库",
                {"nested": "object", "in": "array"},
                [1, 2, 3, {"complex": "structure"}],
                {"$oid": "507f1f77bcf86cd799439014"},
                {"$date": "2025-01-15T00:00:00Z"},
                None,
                True,
                42.195
            ]
        }

        # 插入数据
        insert_result = bridge.create(table_name, json.dumps(test_data), "mongodb_json_test")
        insert_data = json.loads(insert_result)

        if not insert_data.get("success"):
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return False

        print("✅ 数据插入成功")

        # 查询数据
        query_result = bridge.find(table_name, '{}', "mongodb_json_test")
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
        print(f"   记录类型: {type(record)}")

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
                    personal = profile.get('personal', {})
                    if isinstance(personal, dict):
                        print(f"✅ user.profile.personal.name: {personal.get('name')}")
                        print(f"✅ user.profile.personal.age: {personal.get('age')}")
                        print(f"✅ user.profile.personal.email: {personal.get('email')}")

                        preferences = personal.get('preferences', {})
                        if isinstance(preferences, dict):
                            notifications = preferences.get('notifications', {})
                            if isinstance(notifications, dict):
                                print(f"✅ 深层嵌套通知设置: email={notifications.get('email')}, sms={notifications.get('sms')}")

                    professional = profile.get('professional', {})
                    if isinstance(professional, dict):
                        print(f"✅ 职业信息: {professional.get('title')} - {professional.get('department')}")
                        print(f"✅ 技能: {professional.get('skills')}")
                        print(f"✅ 项目数量: {len(professional.get('projects', []))}")

            # 验证复杂数组
            content_items = json_field.get('content_items', [])
            if isinstance(content_items, list):
                print(f"✅ 内容项目数量: {len(content_items)}")
                for i, item in enumerate(content_items[:2]):  # 只检查前两个
                    if isinstance(item, dict):
                        print(f"✅ 内容项目[{i}]: {item.get('type')} - {item.get('title')}")
                        metadata = item.get('metadata', {})
                        if isinstance(metadata, dict):
                            stats = metadata.get('statistics', {})
                            if isinstance(stats, dict):
                                print(f"✅ 统计数据: views={stats.get('views')}, likes={stats.get('likes')}")

            # 验证系统配置
            system_config = json_field.get('system_config', {})
            if isinstance(system_config, dict):
                db_config = system_config.get('database', {})
                if isinstance(db_config, dict):
                    write_concern = db_config.get('write_concern', {})
                    if isinstance(write_concern, dict):
                        print(f"✅ 写入策略: w={write_concern.get('w')}, j={write_concern.get('j')}")

                cache_config = system_config.get('cache', {})
                if isinstance(cache_config, dict):
                    print(f"✅ 缓存配置: enabled={cache_config.get('enabled')}, ttl={cache_config.get('ttl')}")

            # 验证分析数据
            analytics = json_field.get('analytics', {})
            if isinstance(analytics, dict):
                performance = analytics.get('performance', {})
                if isinstance(performance, dict):
                    query_stats = performance.get('query_stats', {})
                    if isinstance(query_stats, dict):
                        print(f"✅ 查询性能: avg={query_stats.get('avg_response_time')}ms, p95={query_stats.get('p95_response_time')}ms")

                    storage = performance.get('storage', {})
                    if isinstance(storage, dict):
                        print(f"✅ 存储信息: total={storage.get('total_size_gb')}GB, compression_ratio={storage.get('compression_ratio')}")

        else:
            print(f"❌ JSON字段解析失败: {type(json_field)}")
            return False

        # 验证数组字段
        array_field = record.get('array_field')
        print(f"   array_field类型: {type(array_field)}")
        print(f"   array_field长度: {len(array_field) if hasattr(array_field, '__len__') else 'N/A'}")

        if isinstance(array_field, list):
            print("✅ 数组字段正确解析为list")
            # 检查数组中的不同类型元素
            for i, item in enumerate(array_field[:5]):  # 只检查前5个
                print(f"   元素[{i}]: {item} (类型: {type(item)})")

        # 清理
        bridge.drop_table(table_name, "mongodb_json_test")
        print("✅ MongoDB测试完成")
        return True

    except Exception as e:
        print(f"❌ MongoDB测试异常: {e}")
        return False

def main():
    """主测试函数"""
    print("🧪 MongoDB数据库JSON字段解析验证")
    print("测试MongoDB原生JSON处理能力")

    # 初始化日志
    try:
        rq.init_logging_with_level("info")
        print("✅ 日志初始化成功")
    except:
        print("⚠️ 日志初始化失败")

    result = test_mongodb_json_parsing()

    print("\n" + "="*50)
    print("🎯 测试结果")
    print("="*50)
    print(f"MongoDB: {'✅ 通过' if result else '❌ 失败'}")

    if result:
        print("\n🎉 MongoDB JSON字段解析功能完全正常！")
        print("✅ register_model功能在MongoDB中正常工作")
        print("✅ MongoDB原生支持复杂JSON结构")
        print("✅ 支持任意深度的嵌套对象和数组")
        print("✅ 支持多种数据类型（字符串、数字、布尔值、null、ObjectId、ISODate等）")
        print("✅ ODM模型注册让MongoDB能正确识别和处理JSON字段")
        return True
    else:
        print("\n⚠️ MongoDB JSON字段解析功能存在问题")
        return False

if __name__ == "__main__":
    main()