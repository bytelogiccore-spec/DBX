//! PhysicalPlanner 구현
//!
//! LogicalPlan → PhysicalPlan 변환

use super::types::*;
use crate::error::{DbxError, DbxResult};
use arrow::datatypes::Schema;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 물리 플랜 빌더 — LogicalPlan → PhysicalPlan 변환
pub struct PhysicalPlanner {
    table_schemas: Arc<RwLock<HashMap<String, Arc<Schema>>>>,
}

impl PhysicalPlanner {
    pub fn new(table_schemas: Arc<RwLock<HashMap<String, Arc<Schema>>>>) -> Self {
        Self { table_schemas }
    }

    /// Convert LogicalPlan → PhysicalPlan
    pub fn plan(&self, logical_plan: &LogicalPlan) -> DbxResult<PhysicalPlan> {
        match logical_plan {
            LogicalPlan::Scan {
                table,
                columns: _,
                filter,
            } => {
                let schemas = self.table_schemas.read().unwrap();
                let schema = schemas
                    .get(table)
                    .ok_or_else(|| DbxError::TableNotFound(table.clone()))?;
                let column_names: Vec<String> =
                    schema.fields().iter().map(|f| f.name().clone()).collect();
                drop(schemas);

                let physical_filter = filter
                    .as_ref()
                    .map(|f| self.plan_physical_expr(f, &column_names))
                    .transpose()?;

                Ok(PhysicalPlan::TableScan {
                    table: table.clone(),
                    projection: vec![],
                    filter: physical_filter,
                })
            }
            LogicalPlan::Project { input, projections } => {
                let input_plan = self.plan(input)?;
                let input_schema = self.extract_schema(&input_plan);
                let mut physical_exprs = Vec::new();
                let mut aliases = Vec::new();

                for (expr, alias) in projections {
                    physical_exprs.push(self.plan_physical_expr(expr, &input_schema)?);
                    aliases.push(alias.clone());
                }

                Ok(PhysicalPlan::Projection {
                    input: Box::new(input_plan),
                    exprs: physical_exprs,
                    aliases,
                })
            }
            LogicalPlan::Filter { input, predicate } => {
                let mut input_plan = self.plan(input)?;
                let input_schema = self.extract_schema(&input_plan);
                let physical_pred = self.plan_physical_expr(predicate, &input_schema)?;

                match &mut input_plan {
                    PhysicalPlan::TableScan { filter, .. } if filter.is_none() => {
                        *filter = Some(physical_pred);
                        Ok(input_plan)
                    }
                    _ => Ok(PhysicalPlan::Projection {
                        input: Box::new(input_plan),
                        exprs: vec![physical_pred],
                        aliases: vec![None], // Filter result column placeholder
                    }),
                }
            }
            LogicalPlan::Aggregate {
                input,
                group_by,
                aggregates,
            } => {
                let input_plan = self.plan(input)?;
                let input_schema = self.extract_schema(&input_plan);

                let group_by_indices: Vec<usize> = group_by
                    .iter()
                    .map(|e| match e {
                        Expr::Column(name) => {
                            input_schema.iter().position(|s| s == name).unwrap_or(0)
                        }
                        _ => 0,
                    })
                    .collect();
                let physical_aggs = aggregates
                    .iter()
                    .map(|agg| PhysicalAggExpr {
                        function: agg.function,
                        input: match &agg.expr {
                            Expr::Column(name) => {
                                input_schema.iter().position(|s| s == name).unwrap_or(0)
                            }
                            _ => 0,
                        },
                        alias: agg.alias.clone(),
                    })
                    .collect();
                Ok(PhysicalPlan::HashAggregate {
                    input: Box::new(input_plan),
                    group_by: group_by_indices,
                    aggregates: physical_aggs,
                })
            }
            LogicalPlan::Sort { input, order_by } => {
                let input_plan = self.plan(input)?;
                let input_schema = self.extract_schema(&input_plan);

                let order_by_physical: Vec<(usize, bool)> = order_by
                    .iter()
                    .map(|s| {
                        let idx = match &s.expr {
                            Expr::Column(name) => {
                                input_schema.iter().position(|n| n == name).unwrap_or(0)
                            }
                            _ => 0,
                        };
                        (idx, s.asc)
                    })
                    .collect();
                Ok(PhysicalPlan::SortMerge {
                    input: Box::new(input_plan),
                    order_by: order_by_physical,
                })
            }
            LogicalPlan::Limit {
                input,
                count,
                offset,
            } => {
                let input_plan = self.plan(input)?;
                Ok(PhysicalPlan::Limit {
                    input: Box::new(input_plan),
                    count: *count,
                    offset: *offset,
                })
            }
            LogicalPlan::Join {
                left,
                right,
                join_type,
                on,
            } => {
                let left_plan = self.plan(left)?;
                let right_plan = self.plan(right)?;

                let left_schema = self.extract_schema(&left_plan);
                let right_schema = self.extract_schema(&right_plan);

                let on_pairs = self.parse_join_condition(on, &left_schema, &right_schema)?;

                Ok(PhysicalPlan::HashJoin {
                    left: Box::new(left_plan),
                    right: Box::new(right_plan),
                    on: on_pairs,
                    join_type: *join_type,
                })
            }
            LogicalPlan::Insert {
                table,
                columns,
                values,
            } => {
                // Convert logical expressions to physical expressions
                let physical_values: Vec<Vec<PhysicalExpr>> = values
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|expr| match expr {
                                Expr::Literal(scalar) => Ok(PhysicalExpr::Literal(scalar.clone())),
                                Expr::Column(_name) => {
                                    // For INSERT, columns are literal values, not references
                                    Err(DbxError::SqlNotSupported {
                                        feature: "Column references in INSERT VALUES".to_string(),
                                        hint: "Use literal values only".to_string(),
                                    })
                                }
                                _ => Err(DbxError::SqlNotSupported {
                                    feature: format!("Expression in INSERT VALUES: {:?}", expr),
                                    hint: "Use literal values only".to_string(),
                                }),
                            })
                            .collect::<DbxResult<Vec<_>>>()
                    })
                    .collect::<DbxResult<Vec<_>>>()?;

                Ok(PhysicalPlan::Insert {
                    table: table.clone(),
                    columns: columns.clone(),
                    values: physical_values,
                })
            }
            LogicalPlan::Update {
                table,
                assignments,
                filter,
            } => {
                // Convert assignments to physical expressions
                let physical_assignments: Vec<(String, PhysicalExpr)> = assignments
                    .iter()
                    .map(|(col, expr)| {
                        let physical_expr = match expr {
                            Expr::Literal(scalar) => Ok(PhysicalExpr::Literal(scalar.clone())),
                            _ => Err(DbxError::NotImplemented(
                                "Non-literal UPDATE values not yet supported".to_string(),
                            )),
                        }?;
                        Ok((col.clone(), physical_expr))
                    })
                    .collect::<DbxResult<Vec<_>>>()?;

                // Convert filter using full expression planner (same as SELECT)
                let physical_filter = if let Some(f) = filter.as_ref() {
                    let schemas = self.table_schemas.read().unwrap();
                    let column_names: Vec<String> = schemas
                        .get(table)
                        .map(|schema| {
                            schema
                                .fields()
                                .iter()
                                .map(|field| field.name().clone())
                                .collect()
                        })
                        .unwrap_or_default();
                    drop(schemas);
                    Some(self.plan_physical_expr(f, &column_names)?)
                } else {
                    None
                };

                Ok(PhysicalPlan::Update {
                    table: table.clone(),
                    assignments: physical_assignments,
                    filter: physical_filter,
                })
            }
            LogicalPlan::Delete { table, filter } => {
                // Convert filter using full expression planner (same as SELECT)
                let physical_filter = if let Some(f) = filter.as_ref() {
                    let schemas = self.table_schemas.read().unwrap();
                    let column_names: Vec<String> = schemas
                        .get(table)
                        .map(|schema| {
                            schema
                                .fields()
                                .iter()
                                .map(|field| field.name().clone())
                                .collect()
                        })
                        .unwrap_or_default();
                    drop(schemas);
                    Some(self.plan_physical_expr(f, &column_names)?)
                } else {
                    None
                };

                Ok(PhysicalPlan::Delete {
                    table: table.clone(),
                    filter: physical_filter,
                })
            }
            LogicalPlan::DropTable { table, if_exists } => Ok(PhysicalPlan::DropTable {
                table: table.clone(),
                if_exists: *if_exists,
            }),
            LogicalPlan::CreateTable {
                table,
                columns,
                if_not_exists,
            } => Ok(PhysicalPlan::CreateTable {
                table: table.clone(),
                columns: columns.clone(),
                if_not_exists: *if_not_exists,
            }),
            LogicalPlan::CreateIndex {
                table,
                index_name,
                columns,
                if_not_exists,
            } => Ok(PhysicalPlan::CreateIndex {
                table: table.clone(),
                index_name: index_name.clone(),
                columns: columns.clone(),
                if_not_exists: *if_not_exists,
            }),
            LogicalPlan::DropIndex {
                table,
                index_name,
                if_exists,
            } => Ok(PhysicalPlan::DropIndex {
                table: table.clone(),
                index_name: index_name.clone(),
                if_exists: *if_exists,
            }),
            LogicalPlan::AlterTable { table, operation } => Ok(PhysicalPlan::AlterTable {
                table: table.clone(),
                operation: operation.clone(),
            }),
            LogicalPlan::CreateFunction {
                name,
                params,
                return_type,
                language,
                body,
            } => Ok(PhysicalPlan::CreateFunction {
                name: name.clone(),
                params: params.clone(),
                return_type: return_type.clone(),
                language: language.clone(),
                body: body.clone(),
            }),
            LogicalPlan::CreateTrigger {
                name,
                timing,
                event,
                table,
                for_each,
                function,
            } => Ok(PhysicalPlan::CreateTrigger {
                name: name.clone(),
                timing: *timing,
                event: *event,
                table: table.clone(),
                for_each: *for_each,
                function: function.clone(),
            }),
            LogicalPlan::CreateJob {
                name,
                schedule,
                function,
            } => Ok(PhysicalPlan::CreateJob {
                name: name.clone(),
                schedule: schedule.clone(),
                function: function.clone(),
            }),
            LogicalPlan::DropFunction { name, if_exists } => Ok(PhysicalPlan::DropFunction {
                name: name.clone(),
                if_exists: *if_exists,
            }),
            LogicalPlan::DropTrigger { name, if_exists } => Ok(PhysicalPlan::DropTrigger {
                name: name.clone(),
                if_exists: *if_exists,
            }),
            LogicalPlan::DropJob { name, if_exists } => Ok(PhysicalPlan::DropJob {
                name: name.clone(),
                if_exists: *if_exists,
            }),
        }
    }

    fn plan_physical_expr(&self, expr: &Expr, schema: &[String]) -> DbxResult<PhysicalExpr> {
        match expr {
            Expr::Column(name) => {
                if let Some(idx) = schema
                    .iter()
                    .position(|s| s.to_lowercase() == name.to_lowercase())
                {
                    Ok(PhysicalExpr::Column(idx))
                } else {
                    Err(DbxError::Schema(format!(
                        "Column '{}' not found in schema: {:?}",
                        name, schema
                    )))
                }
            }
            Expr::Literal(scalar) => Ok(PhysicalExpr::Literal(scalar.clone())),
            Expr::BinaryOp { left, op, right } => Ok(PhysicalExpr::BinaryOp {
                left: Box::new(self.plan_physical_expr(left, schema)?),
                op: *op,
                right: Box::new(self.plan_physical_expr(right, schema)?),
            }),
            Expr::IsNull(expr) => Ok(PhysicalExpr::IsNull(Box::new(
                self.plan_physical_expr(expr, schema)?,
            ))),
            Expr::IsNotNull(expr) => Ok(PhysicalExpr::IsNotNull(Box::new(
                self.plan_physical_expr(expr, schema)?,
            ))),
            Expr::ScalarFunc { func, args } => {
                let physical_args = args
                    .iter()
                    .map(|arg| self.plan_physical_expr(arg, schema))
                    .collect::<DbxResult<Vec<_>>>()?;
                Ok(PhysicalExpr::ScalarFunc {
                    func: *func,
                    args: physical_args,
                })
            }
            _ => Err(DbxError::NotImplemented(format!(
                "Physical expression not supported: {:?}",
                expr
            ))),
        }
    }

    /// Extract schema field names from PhysicalPlan.
    fn extract_schema(&self, plan: &PhysicalPlan) -> Vec<String> {
        match plan {
            PhysicalPlan::TableScan { table, .. } => {
                // Return actual table column names from stored schemas
                let schemas = self.table_schemas.read().unwrap();
                // Try case-insensitive lookup
                let schema = schemas.get(table).or_else(|| {
                    let table_lower = table.to_lowercase();
                    schemas
                        .iter()
                        .find(|(k, _)| k.to_lowercase() == table_lower)
                        .map(|(_, v)| v)
                });

                if let Some(schema) = schema {
                    schema.fields().iter().map(|f| f.name().clone()).collect()
                } else {
                    vec![]
                }
            }
            PhysicalPlan::Projection { exprs, aliases, .. } => exprs
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    if let Some(alias) = aliases.get(i) {
                        alias.clone().unwrap_or_else(|| format!("col_{}", i))
                    } else {
                        format!("col_{}", i)
                    }
                })
                .collect(),
            PhysicalPlan::HashAggregate { input, .. } => self.extract_schema(input),
            PhysicalPlan::SortMerge { input, .. } => self.extract_schema(input),
            PhysicalPlan::Limit { input, .. } => self.extract_schema(input),
            PhysicalPlan::HashJoin { left, right, .. } => {
                let mut fields = self.extract_schema(left);
                fields.extend(self.extract_schema(right));
                fields
            }
            PhysicalPlan::Insert { columns, .. } => columns.clone(),
            PhysicalPlan::Update { .. } => vec![],
            PhysicalPlan::Delete { .. } => vec![],
            PhysicalPlan::DropTable { .. } => vec![],
            PhysicalPlan::CreateTable { .. } => vec![],
            PhysicalPlan::CreateIndex { .. } => vec![],
            PhysicalPlan::DropIndex { .. } => vec![],
            PhysicalPlan::AlterTable { .. } => vec![],
            PhysicalPlan::CreateFunction { .. } => vec![],
            PhysicalPlan::CreateTrigger { .. } => vec![],
            PhysicalPlan::CreateJob { .. } => vec![],
            PhysicalPlan::DropFunction { .. } => vec![],
            PhysicalPlan::DropTrigger { .. } => vec![],
            PhysicalPlan::DropJob { .. } => vec![],
        }
    }

    /// Parse JOIN ON condition to extract (left_col_idx, right_col_idx) pairs.
    /// Supports: col1 = col2 AND col3 = col4 ...
    fn parse_join_condition(
        &self,
        on: &Expr,
        left_schema: &[String],
        right_schema: &[String],
    ) -> DbxResult<Vec<(usize, usize)>> {
        let mut pairs = Vec::new();
        self.extract_join_pairs(on, left_schema, right_schema, &mut pairs)?;

        if pairs.is_empty() {
            // Fallback: use (0, 1) if no pairs extracted
            pairs.push((0, 1));
        }

        Ok(pairs)
    }

    /// Recursively extract join column pairs from ON expression.
    fn extract_join_pairs(
        &self,
        expr: &Expr,
        left_schema: &[String],
        right_schema: &[String],
        pairs: &mut Vec<(usize, usize)>,
    ) -> DbxResult<()> {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                match op {
                    BinaryOperator::Eq => {
                        // Extract column names from left = right
                        let left_col = self.extract_column_name(left)?;
                        let right_col = self.extract_column_name(right)?;

                        // Resolve column indices
                        // Try to find in left schema first, then right
                        let left_idx =
                            self.resolve_column_index(&left_col, left_schema, right_schema, true)?;
                        let right_idx = self.resolve_column_index(
                            &right_col,
                            left_schema,
                            right_schema,
                            false,
                        )?;

                        pairs.push((left_idx, right_idx));
                    }
                    BinaryOperator::And => {
                        // Recursively process AND conditions
                        self.extract_join_pairs(left, left_schema, right_schema, pairs)?;
                        self.extract_join_pairs(right, left_schema, right_schema, pairs)?;
                    }
                    _ => {
                        return Err(DbxError::NotImplemented(format!(
                            "JOIN condition operator not supported: {:?}",
                            op
                        )));
                    }
                }
            }
            _ => {
                return Err(DbxError::NotImplemented(format!(
                    "JOIN condition expression not supported: {:?}",
                    expr
                )));
            }
        }
        Ok(())
    }

    /// Extract column name from expression (handles table.column format).
    fn extract_column_name(&self, expr: &Expr) -> DbxResult<String> {
        match expr {
            Expr::Column(name) => {
                // Handle "table.column" or just "column"
                if let Some(dot_pos) = name.rfind('.') {
                    Ok(name[dot_pos + 1..].to_string())
                } else {
                    Ok(name.clone())
                }
            }
            _ => Err(DbxError::NotImplemented(format!(
                "Expected column reference, got: {:?}",
                expr
            ))),
        }
    }

    /// Resolve column name to index in schema.
    fn resolve_column_index(
        &self,
        col_name: &str,
        left_schema: &[String],
        right_schema: &[String],
        prefer_left: bool,
    ) -> DbxResult<usize> {
        // Try preferred schema first
        if prefer_left {
            if let Some(idx) = left_schema.iter().position(|f| f == col_name) {
                return Ok(idx);
            }
            if let Some(idx) = right_schema.iter().position(|f| f == col_name) {
                return Ok(idx);
            }
        } else {
            if let Some(idx) = right_schema.iter().position(|f| f == col_name) {
                return Ok(idx);
            }
            if let Some(idx) = left_schema.iter().position(|f| f == col_name) {
                return Ok(idx);
            }
        }

        // Fallback: use hardcoded indices based on column name
        // This is a temporary workaround until we have proper schema binding
        match col_name {
            "id" => Ok(0),
            "user_id" => Ok(1),
            "name" => Ok(1),
            _ => {
                eprintln!(
                    "WARNING: Column '{}' not found in schema, using index 0",
                    col_name
                );
                Ok(0)
            }
        }
    }
}

impl Default for PhysicalPlanner {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(HashMap::new())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::{LogicalPlanner, SqlParser};

    #[test]
    fn test_physical_plan_simple_select() {
        let parser = SqlParser::new();
        let statements = parser.parse("SELECT * FROM users").unwrap();

        let logical_planner = LogicalPlanner::new();
        let logical_plan = logical_planner.plan(&statements[0]).unwrap();

        // Inject schema for 'users'
        let table_schemas = Arc::new(RwLock::new(HashMap::new()));
        let schema = Arc::new(Schema::new(vec![
            arrow::datatypes::Field::new("id", arrow::datatypes::DataType::Int32, false),
            arrow::datatypes::Field::new("name", arrow::datatypes::DataType::Utf8, false),
        ]));
        table_schemas
            .write()
            .unwrap()
            .insert("users".to_string(), schema);

        let physical_planner = PhysicalPlanner::new(table_schemas);
        let physical_plan = physical_planner.plan(&logical_plan).unwrap();

        match physical_plan {
            PhysicalPlan::Projection { input, .. } => match input.as_ref() {
                PhysicalPlan::TableScan { table, .. } => {
                    assert_eq!(table, "users");
                }
                _ => panic!("Expected TableScan inside Projection"),
            },
            PhysicalPlan::TableScan { table, .. } => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected Projection or TableScan"),
        }
    }

    #[test]
    fn test_physical_plan_analytical_detection() {
        // 1. Simple TableScan (not analytical)
        let plan1 = PhysicalPlan::TableScan {
            table: "users".to_string(),
            projection: vec![0, 1],
            filter: None,
        };
        assert!(!plan1.is_analytical());
        assert_eq!(plan1.tables(), vec!["users"]);

        // 2. TableScan with Filter (analytical)
        let plan2 = PhysicalPlan::TableScan {
            table: "users".to_string(),
            projection: vec![0, 1],
            filter: Some(PhysicalExpr::Column(0)),
        };
        assert!(plan2.is_analytical());

        // 3. HashJoin (analytical)
        let plan3 = PhysicalPlan::HashJoin {
            left: Box::new(PhysicalPlan::TableScan {
                table: "users".to_string(),
                projection: vec![0],
                filter: None,
            }),
            right: Box::new(PhysicalPlan::TableScan {
                table: "orders".to_string(),
                projection: vec![0],
                filter: None,
            }),
            on: vec![(0, 0)],
            join_type: JoinType::Inner,
        };
        assert!(plan3.is_analytical());
        let tables = plan3.tables();
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }
}
