use super::*;
use crate::auto_api::{MutationRoot, QueryRoot};
use actix_web::{test, App};
use async_graphql::{EmptySubscription, Schema};
use dotenvy::dotenv;
use serde_json::json;

#[actix_web::test]
async fn test() {
    dotenv().ok();
    let _guard = db::start_test().await.unwrap();
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();
    let _app = test::init_service(
        App::new()
            .wrap_fn(|req, srv| {
                req.extensions_mut().insert(Ctx::new());
                if let Some(claims) = auth::retrieve_claims(req.request()) {
                    req.extensions_mut().insert(claims);
                }
                srv.call(req)
            })
            .service(
                web::resource("/gql")
                    .guard(guard::Post())
                    .app_data(Data::new(schema.clone()))
                    .to(auto_api::graphql),
            ),
    )
    .await;
    // db_data::seeder::SeedSchema::seed(include_str!("seed.yml"))
    //     .await
    //     .unwrap();

    // let token = auth::create_jwt("test".to_string(), auth::Role::Admin);
    // let query = r#" GraphQL Query "#;
    // let req = test::TestRequest::post()
    //     .uri("/gql")
    //     .insert_header(("Authorization", format!("Bearer {}", &token)))
    //     .set_json(json!({"query":query, "variables": {}}))
    //     .to_request();

    // let resp: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    // assert_eq!(resp["data"]....., "");
}
@{-"\n"}@