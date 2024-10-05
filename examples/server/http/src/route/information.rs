use std::{collections::HashMap, hash::Hash};

use axum::{
    body::{to_bytes, Body, HttpBody},
    extract::{ConnectInfo, Host, NestedPath, OriginalUri, Request},
    http::{
        request::Parts,
        uri::{Builder, Scheme},
        HeaderMap, Method, Uri, Version,
    },
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
        .route("/", any(information))
        .route("/*path", any(information))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InformationResponse {
    #[serde(default, with = "scheme")]
    pub scheme: Option<Scheme>,
    #[serde(default)]
    pub hostname: String,
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
    pub body: String, // TODO do not String
}

// TODO with = "http_serde::scheme" doesn't supported https://gitlab.com/kornelski/http-serde/-/issues/1
mod scheme {
    use super::*;
    pub fn serialize<S>(value: &Option<Scheme>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.as_ref().map(|scheme| scheme.to_string()).serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Scheme>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|scheme| scheme.parse())
            .transpose()
            .map_err(serde::de::Error::custom)
    }
}

#[tracing::instrument]
pub async fn information(
    // Scheme(scheme): Scheme, // TODO cannot get scheme in axum handler now https://github.com/tokio-rs/axum/pull/2507
    Host(hostname): Host,
    OriginalUri(original_uri): OriginalUri,
    request: Request,
) -> Result<Json<InformationResponse>> {
    let scheme = None;
    let (Parts { method, uri: _, version, headers, .. }, b) = request.into_parts();
    let uri = Builder::from(original_uri)
        .scheme(scheme.clone().unwrap_or(Scheme::HTTP))
        // .authority(hostname.to_string())
        .build()
        .map_err(AppError::<Unreachable>::wrap)?;
    let path = uri.path().to_string();
    let query = parse_query(uri.query().unwrap_or_default())?;
    let body = parse_body(b).await?;
    Ok(Json(InformationResponse { scheme, hostname, method, uri, path, query, version, headers, body }))
}

pub fn parse_query(query: &str) -> Result<HashMap<String, Vec<Value>>> {
    // TODO want to use serde_qs but it has the issue https://github.com/samscott89/serde_qs/issues/77
    //      serde_qs maybe can parse as HashMap or Struct only, so cannot parse as Value or Vec<(String, Value)>
    //      and serde_qs do not allow multiple values for the same key even if use multi map https://github.com/samscott89/serde_qs/blob/b7278b73c637f7c427be762082fee5938ba0c023/src/de/parse.rs#L38
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
    use serde_json::json;

    use crate::{
        error::{
            kind::{BadRequest, Kind},
            ErrorResponseInner, APP_DEFAULT_ERROR_CODE,
        },
        route::{app_with, tests::call_with_assert},
    };

    use super::*;

    #[tokio::test]
    async fn test_information_get() {
        let mut app = app_with(Default::default());

        let req = Request::builder().uri("http://localhost:3000/information/").body(Body::empty()).unwrap();
        call_with_assert(
            &mut app,
            req,
            StatusCode::OK,
            InformationResponse {
                hostname: "localhost".to_string(),
                uri: Uri::from_static("http://localhost:3000/information"),
                path: "/information".to_string(),
                ..Default::default()
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_information_post() {
        let mut app = app_with(Default::default());

        let req = Request::builder()
            .method(Method::POST)
            .uri("http://localhost:3000/information/post/to?type=txt")
            .body(Body::from("body"))
            .unwrap();
        call_with_assert(
            &mut app,
            req,
            StatusCode::OK,
            InformationResponse {
                scheme: None,
                hostname: "localhost".to_string(),
                method: Method::POST,
                uri: Uri::from_static("http://localhost:3000/information/post/to?type=txt"),
                path: "/information/post/to".to_string(),
                query: vec![("type".to_string(), vec![json!("txt")])].into_iter().collect(),
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: "body".to_string(),
            },
        )
        .await;
    }
}
