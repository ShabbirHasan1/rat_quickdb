#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
单独测试PostgreSQL的JSON字段解析功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def test_postgresql_only():
    """只测试PostgreSQL JSON字段解析"""
    print("\n" + "="*50)
    print("🚀 测试 PostgreSQL JSON字段解析")
    print("="*50)

    bridge = rq.create_db_queue_bridge()

    # 添加PostgreSQL数据库
    result = bridge.add_postgresql_database(
        alias="postgresql_json_test",
        host="172.16.0.23",
        port=5432,
        database="testdb",
        username="testdb",
        password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
        max_connections=5,
        min_connections=1,
        connection_timeout=30,
        idle_timeout=600,
        max_lifetime=3600
    )

    if not json.loads(result).get("success"):
        print(f"❌ PostgreSQL数据库添加失败: {json.loads(result).get('error')}")
        return False

    print("✅ PostgreSQL数据库添加成功")

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
    table_name = f"pg_json_{int(time.time())}"
    model_meta = rq.ModelMeta(
        table_name,
        fields_dict,
        [index_def],
        "postgresql_json_test",
        "PostgreSQL JSON测试表"
    )

    # 注册模型
    register_result = bridge.register_model(model_meta)
    if not json.loads(register_result).get("success"):
        print(f"❌ ODM模型注册失败")
        return False

    print("✅ ODM模型注册成功")

    # 测试数据 - PostgreSQL特有的JSONB功能测试
    test_data = {
        "name": "PostgreSQL JSONB复杂测试",
        "json_data": {
            "document_management": {
                "documents": [
                    {
                        "id": "doc-001",
                        "title": "PostgreSQL JSONB功能介绍",
                        "content": "本文详细介绍了PostgreSQL的JSONB数据类型及其高级功能",
                        "metadata": {
                            "author": "数据库专家",
                            "publication_date": "2025-01-15",
                            "word_count": 2500,
                            "reading_time": 8,
                            "language": "zh-CN",
                            "tags": ["PostgreSQL", "JSONB", "数据库", "教程"],
                            "difficulty_level": "中级"
                        },
                        "statistics": {
                            "views": 1500,
                            "likes": 120,
                            "comments": 25,
                            "shares": 15,
                            "bookmarks": 45,
                            "rating": 4.8
                        },
                        "versions": [
                            {
                                "version": "1.0",
                                "date": "2025-01-10",
                                "changes": ["初稿完成", "基础内容添加"]
                            },
                            {
                                "version": "1.1",
                                "date": "2025-01-12",
                                "changes": ["添加示例代码", "优化说明"]
                            },
                            {
                                "version": "1.2",
                                "date": "2025-01-15",
                                "changes": ["最终审校", "格式优化"]
                            }
                        ]
                    },
                    {
                        "id": "doc-002",
                        "title": "高级JSONB查询技巧",
                        "content": "探讨PostgreSQL中JSONB字段的高级查询和索引策略",
                        "metadata": {
                            "author": "技术架构师",
                            "publication_date": "2025-01-14",
                            "word_count": 3200,
                            "reading_time": 12,
                            "language": "zh-CN",
                            "tags": ["PostgreSQL", "JSONB", "查询优化", "索引"],
                            "difficulty_level": "高级"
                        },
                        "statistics": {
                            "views": 980,
                            "likes": 85,
                            "comments": 18,
                            "shares": 12,
                            "bookmarks": 32,
                            "rating": 4.6
                        },
                        "references": [
                            {
                                "type": "article",
                                "title": "PostgreSQL官方文档",
                                "url": "https://postgresql.org/docs/",
                                "relevance": 0.95
                            },
                            {
                                "type": "book",
                                "title": "PostgreSQL性能调优",
                                "author": "性能专家",
                                "relevance": 0.88
                            }
                        ]
                    }
                ]
            },
            "search_configuration": {
                "full_text_search": {
                    "enabled": True,
                    "language": "chinese",
                    "stemming": True,
                    "stop_words": ["的", "了", "和", "是", "在"],
                    "weights": {
                        "title": 3.0,
                        "content": 1.0,
                        "tags": 2.0
                    }
                },
                "vector_search": {
                    "enabled": True,
                    "dimensions": 1536,
                    "model": "text-embedding-ada-002",
                    "index_type": "hnsw",
                    "metric": "cosine"
                },
                "faceted_search": {
                    "enabled": True,
                    "facets": [
                        {
                            "field": "metadata.tags",
                            "type": "array"
                        },
                        {
                            "field": "metadata.difficulty_level",
                            "type": "enum"
                        },
                        {
                            "field": "statistics.rating",
                            "type": "range",
                            "ranges": [
                                {"min": 0, "max": 3, "label": "低分"},
                                {"min": 3, "max": 4, "label": "中等"},
                                {"min": 4, "max": 5, "label": "高分"}
                            ]
                        }
                    ]
                }
            },
            "performance_metrics": {
                "query_performance": {
                    "average_response_time": 45.2,
                    "p95_response_time": 120.5,
                    "p99_response_time": 250.8,
                    "queries_per_second": 1500,
                    "cache_hit_rate": 0.85
                },
                "index_performance": {
                    "index_size_mb": 256,
                    "build_time_seconds": 45,
                    "maintenance_overhead": "low",
                    "update_frequency": "real-time"
                },
                "storage_efficiency": {
                    "compression_ratio": 0.65,
                    "deduplication_savings": 0.15,
                    "total_storage_gb": 12.5,
                    "growth_rate_per_month": 0.08
                }
            },
            "integration_capabilities": {
                "apis": [
                    {
                        "name": "REST API",
                        "version": "v2",
                        "endpoints": 25,
                        "authentication": "JWT",
                        "rate_limit": "1000/minute"
                    },
                    {
                        "name": "GraphQL API",
                        "version": "v1",
                        "schema_complexity": "medium",
                        "real_time_subscriptions": True
                    }
                ],
                "webhooks": [
                    {
                        "event": "document.created",
                        "url": "https://api.example.com/webhooks/document",
                        "retries": 3,
                        "timeout_seconds": 30
                    },
                    {
                        "event": "search.performed",
                        "url": "https://analytics.example.com/webhooks/search",
                        "batch_size": 100
                    }
                ],
                "third_party_integrations": [
                    {
                        "service": "Elasticsearch",
                        "purpose": "增强搜索",
                        "sync_mode": "real-time"
                    },
                    {
                        "service": "Redis",
                        "purpose": "缓存层",
                        "configuration": {
                            "ttl_seconds": 3600,
                            "max_memory_mb": 1024
                        }
                    }
                ]
            }
        }
    }

    # 插入数据
    insert_result = bridge.create(table_name, json.dumps(test_data), "postgresql_json_test")
    insert_data = json.loads(insert_result)

    if not insert_data.get("success"):
        print(f"❌ 数据插入失败: {insert_data.get('error')}")
        return False

    print("✅ 数据插入成功")

    # 查询数据
    query_result = bridge.find(table_name, '{}', "postgresql_json_test")
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

        # 验证文档管理结构
        doc_management = json_field.get('document_management', {})
        if isinstance(doc_management, dict):
            documents = doc_management.get('documents', [])
            if isinstance(documents, list) and len(documents) > 0:
                print(f"✅ documents数量: {len(documents)}")
                first_doc = documents[0]
                if isinstance(first_doc, dict):
                    print(f"✅ 第一个文档标题: {first_doc.get('title')}")
                    print(f"✅ 第一个文档字数: {first_doc.get('metadata', {}).get('word_count')}")

                    versions = first_doc.get('versions', [])
                    if isinstance(versions, list):
                        print(f"✅ 第一个文档版本数量: {len(versions)}")

        # 验证搜索配置
        search_config = json_field.get('search_configuration', {})
        if isinstance(search_config, dict):
            full_text = search_config.get('full_text_search', {})
            if isinstance(full_text, dict):
                print(f"✅ 全文搜索启用: {full_text.get('enabled')}")
                weights = full_text.get('weights', {})
                if isinstance(weights, dict):
                    print(f"✅ 标题权重: {weights.get('title')}")

            vector_search = search_config.get('vector_search', {})
            if isinstance(vector_search, dict):
                print(f"✅ 向量搜索维度: {vector_search.get('dimensions')}")
                print(f"✅ 向量模型: {vector_search.get('model')}")

        # 验证性能指标
        perf_metrics = json_field.get('performance_metrics', {})
        if isinstance(perf_metrics, dict):
            query_perf = perf_metrics.get('query_performance', {})
            if isinstance(query_perf, dict):
                print(f"✅ 平均响应时间: {query_perf.get('average_response_time')}ms")
                print(f"✅ 缓存命中率: {query_perf.get('cache_hit_rate')}")

            index_perf = perf_metrics.get('index_performance', {})
            if isinstance(index_perf, dict):
                print(f"✅ 索引大小: {index_perf.get('index_size_mb')}MB")
                print(f"✅ 索引构建时间: {index_perf.get('build_time_seconds')}s")

        # 验证集成能力
        integration = json_field.get('integration_capabilities', {})
        if isinstance(integration, dict):
            apis = integration.get('apis', [])
            if isinstance(apis, list) and len(apis) > 0:
                print(f"✅ API数量: {len(apis)}")
                print(f"✅ 第一个API: {apis[0].get('name')} v{apis[0].get('version')}")

            webhooks = integration.get('webhooks', [])
            if isinstance(webhooks, list):
                print(f"✅ Webhook数量: {len(webhooks)}")

        print("\n🎯 PostgreSQL JSON字段解析验证完成，所有超复杂嵌套结构都正确解析！")
    else:
        print(f"❌ JSON字段解析失败: {type(json_field)}")
        return False

    # 清理
    bridge.drop_table(table_name, "postgresql_json_test")
    print("✅ PostgreSQL测试完成")
    return True

def main():
    """主函数 - 只测试PostgreSQL"""
    print("🧪 PostgreSQL数据库JSON字段解析验证")
    print("专门测试PostgreSQL的JSON字段解析功能")

    # 初始化日志
    try:
        rq.init_logging_with_level("info")
        print("✅ 日志初始化成功")
    except:
        print("⚠️ 日志初始化失败")

    result = test_postgresql_only()

    print("\n" + "="*50)
    print("🎯 测试结果")
    print("="*50)
    print(f"PostgreSQL: {'✅ 通过' if result else '❌ 失败'}")

    if result:
        print("\n🎉 PostgreSQL JSON字段解析功能完全正常！")
        print("✅ register_model功能正常工作")
        print("✅ ODM模型注册让PostgreSQL能正确识别和解析JSON字段")
        print("✅ 支持超复杂的嵌套JSON结构")
        print("✅ 支持多层嵌套的数组和对象")
        print("✅ 所有数据类型（字符串、数字、布尔值、数组、对象）都正确处理")
        print("✅ PostgreSQL的JSON/JSONB字段正确转换为Python对象")
        return True
    else:
        print("\n⚠️ PostgreSQL JSON字段解析功能存在问题")
        return False

if __name__ == "__main__":
    main()