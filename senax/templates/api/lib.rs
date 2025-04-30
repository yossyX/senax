#[macro_export]
macro_rules! gql_data_find {
    ( $f:ident $p:tt, $repo:expr, $auth:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(obj) => {
                let obj = obj.ok_or_else(|| GqlError::NotFound.extend())?;
                Ok(ResObj::try_from_(&*obj, $auth, None)?)
            }
            Err(e) => {
                if $repo.data_query().should_retry(&e) {
                    $repo.data_query().reset_tx().await;
                    let obj = $f$p
                        .await
                        .map_err(|e| GqlError::server_error($gql_ctx, e))?;
                    let obj = obj.ok_or_else(|| GqlError::NotFound.extend())?;
                    Ok(ResObj::try_from_(&*obj, $auth, None)?)
                } else {
                    Err(GqlError::server_error($gql_ctx, e))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! gql_data_selector {
    ( $f:ident $p:tt, $repo:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(result) => Ok(result),
            Err(e) => {
                if $repo.data_query().should_retry(&e) {
                    $repo.data_query().reset_tx().await;
                    let result = $f$p
                        .await
                        .map_err(|e| GqlError::server_error($gql_ctx, e))?;
                    Ok(result)
                } else {
                    Err(GqlError::server_error($gql_ctx, e))
                }
            }
        }?
    };
}

#[macro_export]
macro_rules! api_data_selector {
    ( $f:ident $p:tt, $repo:expr ) => {
        match $f$p.await {
            Ok(result) => Ok(result),
            Err(e) => {
                if $repo.data_query().should_retry(&e) {
                    $repo.data_query().reset_tx().await;
                    let result = $f$p
                        .await
                        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
                    Ok(result)
                } else {
                    Err(ApiError::InternalServerError(e.to_string()))
                }
            }
        }?
    };
}

#[macro_export]
macro_rules! gql_data_count {
    ( $f:ident $p:tt, $repo:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(count) => Ok(count),
            Err(e) => {
                if $repo.data_query().should_retry(&e) {
                    $repo.data_query().reset_tx().await;
                    let count = $f$p
                        .await
                        .map_err(|e| GqlError::server_error($gql_ctx, e))?;
                    Ok(count)
                } else {
                    Err(GqlError::server_error($gql_ctx, e))
                }
            }
        }
    };
}

pub mod api;
@{-"\n"}@