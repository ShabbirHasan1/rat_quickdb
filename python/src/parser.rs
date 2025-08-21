use pyo3::prelude::*;
use rat_quickdb::types::{
    DataValue, QueryCondition, QueryOperator, QueryConditionGroup, LogicalOperator,
};
use serde_json::{Value as JsonValue};
use std::collections::HashMap;
use zerg_creep::{debug, error, info, warn};

/// 解析器实现
pub struct Parser;

impl Parser {
    /// 解析JSON字符串为数据映射
    pub fn parse_json_to_data_map(&self, json_str: &str) -> Result<HashMap<String, DataValue>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
        if let JsonValue::Object(map) = json_value {
            let mut data_map = HashMap::new();
            for (key, value) in map {
                let data_value = self.parse_data_value(&value)?;
                data_map.insert(key, data_value);
            }
            Ok(data_map)
        } else {
            Err("JSON必须是对象类型".to_string())
        }
    }

    /// 解析查询条件JSON
    /// 解析查询条件JSON字符串（支持条件组合）
    pub fn parse_condition_groups_json(&self, json_str: &str) -> Result<Vec<QueryConditionGroup>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;

        match json_value {
            JsonValue::Array(arr) => {
                let mut groups = Vec::new();
                for item in arr {
                    let group = self.parse_single_condition_group(&item)?;
                    groups.push(group);
                }
                Ok(groups)
            }
            _ => {
                let group = self.parse_single_condition_group(&json_value)?;
                Ok(vec![group])
            }
        }
    }

    /// 解析单个条件组合
    pub fn parse_single_condition_group(&self, json_value: &JsonValue) -> Result<QueryConditionGroup, String> {
        if let JsonValue::Object(ref obj) = json_value {
            // 检查是否是逻辑组合
            if let (Some(operator), Some(conditions)) = (obj.get("operator"), obj.get("conditions")) {
                let logical_op = match operator.as_str() {
                    Some("and") | Some("AND") | Some("And") => LogicalOperator::And,
                    Some("or") | Some("OR") | Some("Or") => LogicalOperator::Or,
                    _ => return Err("不支持的逻辑操作符，支持: and, or".to_string())
                };

                if let JsonValue::Array(conditions_array) = conditions {
                    let mut condition_groups = Vec::new();
                    for condition_item in conditions_array {
                        let group = self.parse_single_condition_group(condition_item)?;
                        condition_groups.push(group);
                    }
                    return Ok(QueryConditionGroup::Group {
                        operator: logical_op,
                        conditions: condition_groups,
                    });
                }
            }

            // 解析为单个条件
            let condition = self.parse_single_condition_from_object(obj)?;
            Ok(QueryConditionGroup::Single(condition))
        } else {
            Err("查询条件必须是对象类型".to_string())
        }
    }

    /// 从对象解析单个条件
    pub fn parse_single_condition_from_object(&self, map: &serde_json::Map<String, JsonValue>) -> Result<QueryCondition, String> {
        let field = map.get("field")
            .and_then(|v| v.as_str())
            .ok_or("field字段必须是字符串")?;
        
        let operator_str = map.get("operator")
            .and_then(|v| v.as_str())
            .ok_or("operator字段必须是字符串")?;
        
        let operator = match operator_str.to_lowercase().as_str() {
            "eq" => QueryOperator::Eq,
            "ne" => QueryOperator::Ne,
            "gt" => QueryOperator::Gt,
            "gte" => QueryOperator::Gte,
            "lt" => QueryOperator::Lt,
            "lte" => QueryOperator::Lte,
            "contains" => QueryOperator::Contains,
            "startswith" | "starts_with" => QueryOperator::StartsWith,
            "endswith" | "ends_with" => QueryOperator::EndsWith,
            "in" => QueryOperator::In,
            "notin" | "not_in" => QueryOperator::NotIn,
            "regex" => QueryOperator::Regex,
            "exists" => QueryOperator::Exists,
            "isnull" | "is_null" => QueryOperator::IsNull,
            "isnotnull" | "is_not_null" => QueryOperator::IsNotNull,
            _ => return Err(format!("不支持的操作符: {}", operator_str)),
        };
        
        let value = self.parse_data_value(map.get("value").ok_or("缺少value字段")?)?;
        
        Ok(QueryCondition {
            field: field.to_string(),
            operator,
            value,
        })
    }

    pub fn parse_conditions_json(&self, json_str: &str) -> Result<Vec<QueryCondition>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
        match json_value {
            // 支持单个查询条件对象格式: {"field": "name", "operator": "Eq", "value": "张三"}
            JsonValue::Object(ref map) if map.contains_key("field") && map.contains_key("operator") && map.contains_key("value") => {
                let field = map.get("field")
                    .and_then(|v| v.as_str())
                    .ok_or("field字段必须是字符串")?;
                
                let operator_str = map.get("operator")
                    .and_then(|v| v.as_str())
                    .ok_or("operator字段必须是字符串")?;
                
                let operator = match operator_str.to_lowercase().as_str() {
                    "eq" => QueryOperator::Eq,
                    "ne" => QueryOperator::Ne,
                    "gt" => QueryOperator::Gt,
                    "gte" => QueryOperator::Gte,
                    "lt" => QueryOperator::Lt,
                    "lte" => QueryOperator::Lte,
                    "contains" => QueryOperator::Contains,
                    "startswith" | "starts_with" => QueryOperator::StartsWith,
                    "endswith" | "ends_with" => QueryOperator::EndsWith,
                    "in" => QueryOperator::In,
                    "notin" | "not_in" => QueryOperator::NotIn,
                    "regex" => QueryOperator::Regex,
                    "exists" => QueryOperator::Exists,
                    "isnull" | "is_null" => QueryOperator::IsNull,
                    "isnotnull" | "is_not_null" => QueryOperator::IsNotNull,
                    _ => return Err(format!("不支持的操作符: {}", operator_str)),
                };
                
                let value = self.parse_data_value(map.get("value").unwrap())?;
                
                let condition = QueryCondition {
                    field: field.to_string(),
                    operator,
                    value,
                };
                
                Ok(vec![condition])
            },
            // 支持多个查询条件数组格式: [{"field": "name", "operator": "Eq", "value": "张三"}, {"field": "age", "operator": "Gt", "value": 20}]
            JsonValue::Array(conditions_array) => {
                let mut conditions = Vec::new();
                
                for condition_value in conditions_array {
                    if let JsonValue::Object(ref condition_map) = condition_value {
                        let field = condition_map.get("field")
                            .and_then(|v| v.as_str())
                            .ok_or("每个查询条件的field字段必须是字符串")?;
                        
                        let operator_str = condition_map.get("operator")
                            .and_then(|v| v.as_str())
                            .ok_or("每个查询条件的operator字段必须是字符串")?;
                        
                        let operator = match operator_str.to_lowercase().as_str() {
                            "eq" => QueryOperator::Eq,
                            "ne" => QueryOperator::Ne,
                            "gt" => QueryOperator::Gt,
                            "gte" => QueryOperator::Gte,
                            "lt" => QueryOperator::Lt,
                            "lte" => QueryOperator::Lte,
                            "contains" => QueryOperator::Contains,
                            "startswith" | "starts_with" => QueryOperator::StartsWith,
                            "endswith" | "ends_with" => QueryOperator::EndsWith,
                            "in" => QueryOperator::In,
                            "notin" | "not_in" => QueryOperator::NotIn,
                            "regex" => QueryOperator::Regex,
                            "exists" => QueryOperator::Exists,
                            "isnull" | "is_null" => QueryOperator::IsNull,
                            "isnotnull" | "is_not_null" => QueryOperator::IsNotNull,
                            _ => return Err(format!("不支持的操作符: {}", operator_str)),
                        };
                        
                        let value = self.parse_data_value(
                            condition_map.get("value")
                                .ok_or("每个查询条件必须包含value字段")?
                        )?;
                        
                        let condition = QueryCondition {
                            field: field.to_string(),
                            operator,
                            value,
                        };
                        
                        conditions.push(condition);
                    } else {
                        return Err("查询条件数组中的每个元素必须是对象".to_string());
                    }
                }
                
                Ok(conditions)
            },
            // 支持简化的键值对格式: {"name": "张三", "age": 20} (默认使用Eq操作符)
            // 以及操作符格式: {"age": {"Gt": 25}, "name": "张三"}
            JsonValue::Object(map) => {
                let mut conditions = Vec::new();
                for (field, value) in map {
                    let condition = match value {
                        // 如果值是对象，检查是否是操作符格式 {"Gt": 25} 或范围查询 {"Gte": 25, "Lte": 30}
                          JsonValue::Object(ref value_obj) if !value_obj.is_empty() => {
                              // 检查是否所有键都是已知操作符
                              let all_operators = value_obj.keys().all(|k| {
                                  matches!(k.to_lowercase().as_str(), 
                                      "eq" | "ne" | "gt" | "gte" | "lt" | "lte" | "contains" | 
                                      "startswith" | "starts_with" | "endswith" | "ends_with" | 
                                      "in" | "notin" | "not_in" | "regex" | "exists" | 
                                      "isnull" | "is_null" | "isnotnull" | "is_not_null")
                              });
                              
                              if all_operators {
                                   // 处理操作符格式，可能有多个操作符（如范围查询）
                                   let mut field_conditions = Vec::new();
                                   for (operator_str, operator_value) in value_obj {
                                       let operator = match operator_str.to_lowercase().as_str() {
                                           "eq" => QueryOperator::Eq,
                                           "ne" => QueryOperator::Ne,
                                           "gt" => QueryOperator::Gt,
                                           "gte" => QueryOperator::Gte,
                                           "lt" => QueryOperator::Lt,
                                           "lte" => QueryOperator::Lte,
                                           "contains" => QueryOperator::Contains,
                                           "startswith" | "starts_with" => QueryOperator::StartsWith,
                                           "endswith" | "ends_with" => QueryOperator::EndsWith,
                                           "in" => QueryOperator::In,
                                           "notin" | "not_in" => QueryOperator::NotIn,
                                           "regex" => QueryOperator::Regex,
                                           "exists" => QueryOperator::Exists,
                                           "isnull" | "is_null" => QueryOperator::IsNull,
                                           "isnotnull" | "is_not_null" => QueryOperator::IsNotNull,
                                           _ => unreachable!(), // 已经检查过了
                                       };
                                       
                                       field_conditions.push(QueryCondition {
                                           field: field.clone(),
                                           operator,
                                           value: self.parse_data_value(operator_value)?,
                                       });
                                   }
                                   
                                   // 将除第一个之外的条件添加到主条件列表中
                                   if field_conditions.len() > 1 {
                                       let mut iter = field_conditions.into_iter();
                                       let first_condition = iter.next().unwrap();
                                       conditions.extend(iter);
                                       first_condition
                                   } else {
                                       field_conditions.into_iter().next().unwrap()
                                   }
                               } else {
                                  // 如果不是操作符格式，当作普通对象值处理
                                  QueryCondition {
                                      field,
                                      operator: QueryOperator::Eq,
                                      value: self.parse_data_value(&value)?,
                                  }
                              }
                          },
                        // 普通值，使用Eq操作符
                        _ => QueryCondition {
                            field,
                            operator: QueryOperator::Eq,
                            value: self.parse_data_value(&value)?,
                        }
                    };
                    conditions.push(condition);
                }
                Ok(conditions)
            },
            _ => Err("查询条件JSON必须是对象或数组类型".to_string()),
        }
    }
    
    /// 解析JSON值为DataValue
    pub fn parse_data_value(&self, json_value: &JsonValue) -> Result<DataValue, String> {
        match json_value {
            JsonValue::String(s) => Ok(DataValue::String(s.clone())),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(DataValue::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(DataValue::Float(f))
                } else {
                    Err(format!("不支持的数字类型: {}", n))
                }
            }
            JsonValue::Bool(b) => Ok(DataValue::Bool(*b)),
            JsonValue::Null => Ok(DataValue::Null),
            JsonValue::Array(arr) => {
                let mut data_values = Vec::new();
                for item in arr {
                    data_values.push(self.parse_data_value(item)?);
                }
                Ok(DataValue::Array(data_values))
            }
            JsonValue::Object(obj) => {
                let mut data_map = HashMap::new();
                for (key, value) in obj {
                    data_map.insert(key.clone(), self.parse_data_value(value)?);
                }
                Ok(DataValue::Object(data_map))
            }
        }
    }
}