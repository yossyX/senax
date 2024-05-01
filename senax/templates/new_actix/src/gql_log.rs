use async_graphql::{
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextExecute, NextParseQuery},
    parser::types::{ExecutableDocument, OperationType, Selection},
    PathSegment, Response, ServerResult, Variables,
};
use std::{fmt::Write, sync::Arc};

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
        let _ctx: &crate::context::Ctx = ctx.data().unwrap();
        let document = next.run(ctx, query, variables).await?;
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

    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let _ctx: &crate::context::Ctx = ctx.data().unwrap();
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
                    warn!(path = path, message = err.message, ctx = _ctx.ctx_no(); "");
                } else {
                    warn!(message = err.message, ctx = _ctx.ctx_no(); "");
                }
            }
        } else {
            info!(target:"response", ctx = _ctx.ctx_no(), response = resp.data.to_string(); "");
        }
        resp
    }
}
