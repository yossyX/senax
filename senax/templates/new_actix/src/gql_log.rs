use async_graphql::{
    extensions::{
        Extension, ExtensionContext, ExtensionFactory, NextExecute, NextParseQuery, NextValidation,
    },
    parser::types::{ExecutableDocument, OperationType, Selection},
    PathSegment, Response, ServerError, ServerResult, ValidationResult, Variables,
};
use std::{fmt::Write, sync::Arc};

use crate::_base::context::Ctx;

pub struct GqlLogger;

impl ExtensionFactory for GqlLogger {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(LoggerExtension)
    }
}

struct LoggerExtension;

#[async_trait::async_trait]
impl Extension for LoggerExtension {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let _ctx: &Ctx = ctx.data().unwrap();
        debug!(ctx = _ctx.ctx_no(), gql_query = query; "");
        let document = match next.run(ctx, query, variables).await {
            Ok(d) => d,
            Err(e) => {
                warn!(ctx = _ctx.ctx_no(); "{}", e.message);
                return Err(e);
            }
        };
        let is_schema = document
            .operations
            .iter()
            .filter(|(_, operation)| operation.node.ty == OperationType::Query)
            .any(|(_, operation)| operation.node.selection_set.node.items.iter().any(|selection| matches!(&selection.node, Selection::Field(field) if field.node.name.node == "__schema")));
        if !is_schema {
            info!(target:"request", ctx = _ctx.ctx_no(), gql = ctx.stringify_execute_doc(&document, variables); "");
        }
        Ok(document)
    }

    async fn validation(
        &self,
        ctx: &ExtensionContext<'_>,
        next: NextValidation<'_>,
    ) -> Result<ValidationResult, Vec<ServerError>> {
        match next.run(ctx).await {
            Ok(r) => Ok(r),
            Err(err_list) => {
                let _ctx: &Ctx = ctx.data().unwrap();
                for err in &err_list {
                    warn!(ctx = _ctx.ctx_no(); "{} {:?}", err.message, err.locations);
                }
                Err(err_list)
            }
        }
    }

    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let _ctx: &Ctx = ctx.data().unwrap();
        let resp = next.run(ctx, operation_name).await;
        if resp.is_err() {
            for err in &resp.errors {
                if !err.path.is_empty() {
                    let mut path = String::new();
                    for (idx, s) in err.path.iter().enumerate() {
                        if idx > 0 {
                            path.push('.');
                        }
                        match s {
                            PathSegment::Index(idx) => {
                                let _ = write!(&mut path, "{}", idx);
                            }
                            PathSegment::Field(name) => {
                                let _ = write!(&mut path, "{}", name);
                            }
                        }
                    }
                    warn!(path = path, ctx = _ctx.ctx_no(); "{}", err.message);
                } else {
                    warn!(ctx = _ctx.ctx_no(); "{}", err.message);
                }
            }
        } else {
            info!(target:"response", ctx = _ctx.ctx_no(), response = resp.data.to_string(); "");
        }
        resp
    }
}
@{-"\n"}@