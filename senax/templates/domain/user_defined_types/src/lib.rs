use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Deserialize,
    Serialize,
    Clone,
    Debug,
    JsonSchema,
    async_graphql::SimpleObject,
    async_graphql::InputObject,
    utoipa::ToSchema,
)]
#[graphql(input_name = "SampleInput")]
#[schema(as = SampleInput)]
pub struct Sample {
    pub a: u64,
    pub b: u64,
}
@{-"\n"}@