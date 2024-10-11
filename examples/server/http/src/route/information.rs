use std::collections::HashMap;

use axum::{
    body::{to_bytes, Body, HttpBody},
    extract::{Host, OriginalUri, Request},
    http::{request::Parts, uri::Scheme, HeaderMap, Method, Uri, Version},
    response::Result,
    routing::any,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    error::{
        kind::{BadRequest, Unreachable},
        AppError,
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
    #[serde(default)]
    pub datetime: Option<DateTime<Utc>>,
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
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub query: Map<String, Value>,
    #[serde(default, with = "http_serde::version")]
    pub version: Version,
    #[serde(default, with = "http_serde::header_map")]
    pub headers: HeaderMap,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub body: Value,
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
    Host(hostname): Host,
    OriginalUri(uri): OriginalUri,
    request: Request,
) -> Result<Json<InformationResponse>> {
    let datetime = if !cfg!(test) { Some(Utc::now()) } else { None }; // TODO should we use cfg! ?
    let scheme = None; // TODO cannot get scheme in axum handler now https://github.com/tokio-rs/axum/pull/2507
    let (Parts { method, uri: _, version, headers, .. }, b) = request.into_parts();
    let path = uri.path().to_string();
    let query = parse_query(uri.query().unwrap_or_default())?;
    let body = parse_body(b).await?;
    Ok(Json(InformationResponse { datetime, scheme, hostname, method, uri, path, query, version, headers, body }))
}

pub fn parse_query(query: &str) -> Result<Map<String, Value>> {
    // TODO want to use serde_qs but it has the issue https://github.com/samscott89/serde_qs/issues/77
    //      serde_qs maybe can parse as HashMap or Struct only, so cannot parse as Value or Vec<(String, Value)>
    //      and serde_qs do not allow multiple values for the same key even if use multi map https://github.com/samscott89/serde_qs/blob/b7278b73c637f7c427be762082fee5938ba0c023/src/de/parse.rs#L38
    let tuples: Vec<(_, Value)> = serde_urlencoded::from_str(query).map_err(AppError::<Unreachable>::wrap)?;
    let mut map = HashMap::new();
    for (q, s) in tuples {
        map.entry(q).or_insert(Vec::new()).push(s);
    }
    Ok(map.into_iter().map(|(k, v)| if let [ref x] = v[..] { (k, x.clone()) } else { (k, Value::Array(v)) }).collect())
}

pub async fn parse_body(b: Body) -> Result<Value> {
    let size = b.size_hint().upper().unwrap_or(b.size_hint().lower()) as usize;
    let bytes = to_bytes(b, size).await.map_err(AppError::<BadRequest>::wrap)?;
    let string = String::from_utf8_lossy(&bytes);

    // TODO content-type based parsing
    if string.is_empty() {
        Ok(Value::Null)
    } else if let Ok(json) = serde_json::from_str(&string) {
        Ok(json)
    } else if let Ok(urlencoded) = parse_query(&string) {
        Ok(Value::Object(urlencoded))
    } else {
        let s = String::from_utf8_lossy(&bytes);
        if s.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(Value::String(s.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{HeaderName, HeaderValue, Request, StatusCode},
    };
    use serde_json::json;

    use crate::route::{app_with, tests::call_with_assert};

    use super::*;

    #[tokio::test]
    async fn test_information_basic() {
        let mut app = app_with(Default::default());

        let req = Request::builder().uri("http://localhost:3000/information").body(Body::empty()).unwrap();
        call_with_assert(
            &mut app,
            req,
            StatusCode::OK,
            InformationResponse {
                hostname: "localhost".to_string(),
                // BUG? in test, include scheme and authority, but server response include only path and query
                uri: Uri::from_static("http://localhost:3000/information"),
                path: "/information".to_string(),
                ..Default::default()
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_information_get() {
        let mut app = app_with(Default::default());

        let req = Request::builder()
            .uri("http://localhost:3000/information/path/to/query/?q=test&k=1&k=2&k=3")
            .body(Body::empty())
            .unwrap();
        call_with_assert(
            &mut app,
            req,
            StatusCode::OK,
            InformationResponse {
                hostname: "localhost".to_string(),
                uri: Uri::from_static("http://localhost:3000/information/path/to/query?q=test&k=1&k=2&k=3"),
                path: "/information/path/to/query".to_string(),
                query: json!({ "q": "test", "k": ["1", "2", "3"] }).as_object().unwrap().clone(),
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
            .uri("http://localhost:3000/information/post/qs/to?type=txt")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("body=body"))
            .unwrap();
        call_with_assert(
            &mut app,
            req,
            StatusCode::OK,
            InformationResponse {
                scheme: None,
                hostname: "localhost".to_string(),
                method: Method::POST,
                uri: Uri::from_static("http://localhost:3000/information/post/qs/to?type=txt"),
                path: "/information/post/qs/to".to_string(),
                query: json!({ "type": "txt" }).as_object().unwrap().clone(),
                version: Version::HTTP_11,
                headers: vec![(
                    HeaderName::from_static("content-type"),
                    HeaderValue::from_static("application/x-www-form-urlencoded"),
                )]
                .into_iter()
                .collect(),
                body: json!({"body": "body"}),
                ..Default::default()
            },
        )
        .await;

        let req = Request::builder()
            .method(Method::POST)
            .uri("http://localhost:3000/information/post/json/to")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name": "json", "key": [1, 2, 3]}"#))
            .unwrap();
        call_with_assert(
            &mut app,
            req,
            StatusCode::OK,
            InformationResponse {
                scheme: None,
                hostname: "localhost".to_string(),
                method: Method::POST,
                uri: Uri::from_static("http://localhost:3000/information/post/json/to"),
                path: "/information/post/json/to".to_string(),
                query: json!({}).as_object().unwrap().clone(),
                version: Version::HTTP_11,
                headers: vec![(HeaderName::from_static("content-type"), HeaderValue::from_static("application/json"))]
                    .into_iter()
                    .collect(),
                body: json!({"name": "json", "key": [1, 2, 3]}),
                ..Default::default()
            },
        )
        .await;
    }
}
