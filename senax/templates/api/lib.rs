#[macro_export]
macro_rules! gql_find {
    ( $f:ident $p:tt, $repo:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(obj) => {
                let obj = obj.ok_or_else(|| GqlError::NotFound.extend())?;
                if !domain::models::FilterFlag::get_flag(obj.as_ref(), "_readable").unwrap_or_default() {
                    return Err(GqlError::Forbidden.extend());
                }
                Ok(ResObj::try_from_(&*obj, None)?)
            }
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
                    let obj = $f$p
                        .await
                        .map_err(|e| GqlError::server_error($gql_ctx, e))?;
                    let obj = obj.ok_or_else(|| GqlError::NotFound.extend())?;
                    if !domain::models::FilterFlag::get_flag(obj.as_ref(), "_readable").unwrap_or_default() {
                        return Err(GqlError::Forbidden.extend());
                    }
                    Ok(ResObj::try_from_(&*obj, None)?)
                } else {
                    Err(GqlError::server_error($gql_ctx, e))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! gql_selector {
    ( $f:ident $p:tt, $repo:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(result) => Ok(result),
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
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
macro_rules! api_selector {
    ( $f:ident $p:tt, $repo:expr ) => {
        match $f$p.await {
            Ok(result) => Ok(result),
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
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
macro_rules! gql_count {
    ( $f:ident $p:tt, $repo:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(count) => Ok(count),
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
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