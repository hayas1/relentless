use std::{collections::HashMap, hash::Hash};

use axum::{
    body::{to_bytes, Body, HttpBody},
    extract::Request,
    http::{request::Parts, HeaderMap, Method, Uri, Version},
    response::Result,
    routing::{any, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{
        kind::{BadRequest, Retriable, Unreachable},
        AppError, Logged,
    },
    state::AppState,
};

pub fn route_information() -> Router<AppState> {
    Router::new()
        // .route("/", any(information))
        .route("/*path", any(information))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformationResponse {
    #[serde(default, with = "http_serde::method")]
    pub method: Method,
    #[serde(default, with = "http_serde::uri")]
    pub uri: Uri,
    #[serde(default)]
    pub path: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub query: HashMap<String, Vec<Value>>,
    #[serde(default, with = "http_serde::version")]
    pub version: Version,
    #[serde(default, with = "http_serde::header_map")]
    pub headers: HeaderMap,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
}

#[tracing::instrument]
pub async fn information(request: Request) -> Result<Json<InformationResponse>> {
    let (Parts { method, uri, version, headers, .. }, b) = request.into_parts();
    let path = uri.path().to_string();
    let query = parse_query(uri.query().unwrap_or_default())?;
    let body = parse_body(b).await?;
    Ok(Json(InformationResponse { method, uri, path, query, version, headers, body }))
}

pub fn parse_query(query: &str) -> Result<HashMap<String, Vec<Value>>> {
    // TODO want to use serde_qs but it has the issue https://github.com/samscott89/serde_qs/issues/77
    //      serde_qs maybe can parse as HashMap or Struct only, so cannot parse as Value or Vec<(String, Value)>
    let tuples: Vec<_> = serde_urlencoded::from_str(query).map_err(AppError::<Unreachable>::wrap)?;
    let mut map = HashMap::new();
    for (q, s) in tuples {
        map.entry(q).or_insert(Vec::new()).push(s);
    }

    Ok(map)
}

pub async fn parse_body(b: Body) -> Result<String> {
    let size = b.size_hint().upper().unwrap_or(b.size_hint().lower()) as usize;
    let bytes = to_bytes(b, size).await.map_err(AppError::<BadRequest>::wrap)?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::{
        error::{
            kind::{BadRequest, Kind},
            ErrorResponseInner, APP_DEFAULT_ERROR_CODE,
        },
        route::{app_with, tests::call_with_assert},
    };

    use super::*;

    #[tokio::test]
    async fn test_information() {
        let mut app = app_with(Default::default());

        let req = Request::builder().uri("/information").body(Body::empty()).unwrap();
        call_with_assert(
            &mut app,
            req,
            StatusCode::OK,
            InformationResponse {
                method: Method::GET,
                uri: Uri::from_static("/information"), // TODO!!!
                path: "/information".to_string(),
                query: HashMap::new(),
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: "".to_string(),
            },
        )
        .await;
    }
}
