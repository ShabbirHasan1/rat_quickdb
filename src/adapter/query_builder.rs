//! SQL查询构建器模块
//! 
//! 提供安全的SQL查询构建功能，防止SQL注入攻击

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use std::collections::HashMap;

/// SQL查询构建器
pub struct SqlQueryBuilder {
    query_type: QueryType,
    table: String,
    fields: Vec<String>,
    conditions: Vec<QueryCondition>,
    joins: Vec<JoinClause>,
    order_by: Vec<OrderClause>,
    group_by: Vec<String>,
    having: Vec<QueryCondition>,
    limit: Option<u64>,
    offset: Option<u64>,
    values: HashMap<String, DataValue>,
    returning_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    pub on_condition: String,
}

#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone)]
pub struct OrderClause {
    pub field: String,
    pub direction: SortDirection,
}

impl SqlQueryBuilder {
    /// 创建新的查询构建器
    pub fn new() -> Self {
        Self {
            query_type: QueryType::Select,
            table: String::new(),
            fields: Vec::new(),
            conditions: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            values: HashMap::new(),
            returning_fields: Vec::new(),
        }
    }

    /// 设置查询类型为SELECT
    pub fn select(mut self, fields: &[&str]) -> Self {
        self.query_type = QueryType::Select;
        self.fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 设置查询类型为INSERT
    pub fn insert(mut self, values: HashMap<String, DataValue>) -> Self {
        self.query_type = QueryType::Insert;
        self.values = values;
        self
    }

    /// 设置查询类型为UPDATE
    pub fn update(mut self, values: HashMap<String, DataValue>) -> Self {
        self.query_type = QueryType::Update;
        self.values = values;
        self
    }

    /// 设置查询类型为DELETE
    pub fn delete(mut self) -> Self {
        self.query_type = QueryType::Delete;
        self
    }

    /// 设置表名
    pub fn from(mut self, table: &str) -> Self {
        self.table = table.to_string();
        self
    }

    /// 添加WHERE条件
    pub fn where_condition(mut self, condition: QueryCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// 添加多个WHERE条件
    pub fn where_conditions(mut self, conditions: &[QueryCondition]) -> Self {
        self.conditions.extend_from_slice(conditions);
        self
    }

    /// 添加JOIN子句
    pub fn join(mut self, join_type: JoinType, table: &str, on_condition: &str) -> Self {
        self.joins.push(JoinClause {
            join_type,
            table: table.to_string(),
            on_condition: on_condition.to_string(),
        });
        self
    }

    /// 添加ORDER BY子句
    pub fn order_by(mut self, field: &str, direction: SortDirection) -> Self {
        self.order_by.push(OrderClause {
            field: field.to_string(),
            direction,
        });
        self
    }

    /// 添加GROUP BY子句
    pub fn group_by(mut self, fields: &[&str]) -> Self {
        self.group_by = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加HAVING条件
    pub fn having(mut self, condition: QueryCondition) -> Self {
        self.having.push(condition);
        self
    }

    /// 设置LIMIT
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 设置OFFSET
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// 设置RETURNING子句（用于INSERT/UPDATE/DELETE）
    pub fn returning(mut self, fields: &[&str]) -> Self {
        self.returning_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 构建SQL查询语句
    pub fn build(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        match self.query_type {
            QueryType::Select => self.build_select(),
            QueryType::Insert => self.build_insert(),
            QueryType::Update => self.build_update(),
            QueryType::Delete => self.build_delete(),
        }
    }

    /// 构建SELECT语句
    fn build_select(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        let fields = if self.fields.is_empty() {
            "*".to_string()
        } else {
            self.fields.join(", ")
        };

        let mut sql = format!("SELECT {} FROM {}", fields, self.table);
        let mut params = Vec::new();

        // 添加JOIN子句
        for join in &self.joins {
            let join_type = match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL OUTER JOIN",
            };
            sql.push_str(&format!(" {} {} ON {}", join_type, join.table, join.on_condition));
        }

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause(&self.conditions)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        // 添加GROUP BY
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        // 添加HAVING
        if !self.having.is_empty() {
            let (having_clause, having_params) = self.build_where_clause(&self.having)?;
            sql.push_str(&format!(" HAVING {}", having_clause));
            params.extend(having_params);
        }

        // 添加ORDER BY
        if !self.order_by.is_empty() {
            let order_clauses: Vec<String> = self.order_by
                .iter()
                .map(|o| {
                    let direction = match o.direction {
                        SortDirection::Asc => "ASC",
                        SortDirection::Desc => "DESC",
                    };
                    format!("{} {}", o.field, direction)
                })
                .collect();
            sql.push_str(&format!(" ORDER BY {}", order_clauses.join(", ")));
        }

        // 添加LIMIT和OFFSET
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        Ok((sql, params))
    }

    /// 构建INSERT语句
    fn build_insert(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        if self.values.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "插入值不能为空".to_string(),
            });
        }

        let columns: Vec<String> = self.values.keys().cloned().collect();
        let placeholders: Vec<String> = (0..columns.len()).map(|_| "?".to_string()).collect();
        let params: Vec<DataValue> = columns.iter().map(|k| self.values[k].clone()).collect();

        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            columns.join(", "),
            placeholders.join(", ")
        );

        // 添加RETURNING子句
        if !self.returning_fields.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning_fields.join(", ")));
        }

        Ok((sql, params))
    }

    /// 构建UPDATE语句
    fn build_update(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        if self.values.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "更新值不能为空".to_string(),
            });
        }

        let set_clauses: Vec<String> = self.values.keys().map(|k| format!("{} = ?", k)).collect();
        let mut params: Vec<DataValue> = self.values.values().cloned().collect();

        let mut sql = format!("UPDATE {} SET {}", self.table, set_clauses.join(", "));

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause(&self.conditions)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        // 添加RETURNING子句
        if !self.returning_fields.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning_fields.join(", ")));
        }

        Ok((sql, params))
    }

    /// 构建DELETE语句
    fn build_delete(&self) -> QuickDbResult<(String, Vec<DataValue>)> {
        if self.table.is_empty() {
            return Err(QuickDbError::QueryError {
                message: "表名不能为空".to_string(),
            });
        }

        let mut sql = format!("DELETE FROM {}", self.table);
        let mut params = Vec::new();

        // 添加WHERE条件
        if !self.conditions.is_empty() {
            let (where_clause, where_params) = self.build_where_clause(&self.conditions)?;
            sql.push_str(&format!(" WHERE {}", where_clause));
            params.extend(where_params);
        }

        // 添加RETURNING子句
        if !self.returning_fields.is_empty() {
            sql.push_str(&format!(" RETURNING {}", self.returning_fields.join(", ")));
        }

        Ok((sql, params))
    }

    /// 构建WHERE子句
    fn build_where_clause(&self, conditions: &[QueryCondition]) -> QuickDbResult<(String, Vec<DataValue>)> {
        if conditions.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        let mut clauses = Vec::new();
        let mut params = Vec::new();

        for condition in conditions {
            match condition.operator {
                QueryOperator::Eq => {
                    clauses.push(format!("{} = ?", condition.field));
                    params.push(condition.value.clone());
                }
                QueryOperator::Ne => {
                    clauses.push(format!("{} != ?", condition.field));
                    params.push(condition.value.clone());
                }
                QueryOperator::Gt => {
                    clauses.push(format!("{} > ?", condition.field));
                    params.push(condition.value.clone());
                }
                QueryOperator::Gte => {
                    clauses.push(format!("{} >= ?", condition.field));
                    params.push(condition.value.clone());
                }
                QueryOperator::Lt => {
                    clauses.push(format!("{} < ?", condition.field));
                    params.push(condition.value.clone());
                }
                QueryOperator::Lte => {
                    clauses.push(format!("{} <= ?", condition.field));
                    params.push(condition.value.clone());
                }
                QueryOperator::Contains => {
                    clauses.push(format!("{} LIKE ?", condition.field));
                    if let DataValue::String(s) = &condition.value {
                        params.push(DataValue::String(format!("%{}%", s)));
                    } else {
                        params.push(condition.value.clone());
                    }
                }
                QueryOperator::StartsWith => {
                    clauses.push(format!("{} LIKE ?", condition.field));
                    if let DataValue::String(s) = &condition.value {
                        params.push(DataValue::String(format!("{}%", s)));
                    } else {
                        params.push(condition.value.clone());
                    }
                }
                QueryOperator::EndsWith => {
                    clauses.push(format!("{} LIKE ?", condition.field));
                    if let DataValue::String(s) = &condition.value {
                        params.push(DataValue::String(format!("%{}", s)));
                    } else {
                        params.push(condition.value.clone());
                    }
                }
                QueryOperator::In => {
                    if let DataValue::Array(values) = &condition.value {
                        let placeholders = vec!["?"; values.len()].join(", ");
                        clauses.push(format!("{} IN ({})", condition.field, placeholders));
                        params.extend(values.clone());
                    } else {
                        return Err(QuickDbError::QueryError {
                            message: "IN 操作符需要数组类型的值".to_string(),
                        });
                    }
                }
                QueryOperator::NotIn => {
                    if let DataValue::Array(values) = &condition.value {
                        let placeholders = vec!["?"; values.len()].join(", ");
                        clauses.push(format!("{} NOT IN ({})", condition.field, placeholders));
                        params.extend(values.clone());
                    } else {
                        return Err(QuickDbError::QueryError {
                            message: "NOT IN 操作符需要数组类型的值".to_string(),
                        });
                    }
                }
                QueryOperator::Regex => {
                    // 不同数据库的正则表达式语法不同，这里使用通用的LIKE
                    clauses.push(format!("{} REGEXP ?", condition.field));
                    params.push(condition.value.clone());
                }
                QueryOperator::Exists => {
                    // 检查字段是否存在（主要用于NoSQL数据库）
                    clauses.push(format!("{} IS NOT NULL", condition.field));
                    // Exists操作符不需要参数值
                }
                QueryOperator::IsNull => {
                    clauses.push(format!("{} IS NULL", condition.field));
                    // IsNull操作符不需要参数值
                }
                QueryOperator::IsNotNull => {
                    clauses.push(format!("{} IS NOT NULL", condition.field));
                    // IsNotNull操作符不需要参数值
                }
            }
        }

        Ok((clauses.join(" AND "), params))
    }
}

impl Default for SqlQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}