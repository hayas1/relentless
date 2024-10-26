use axum::{
    body::Bytes,
    extract::{OriginalUri, Path, Query, Request},
    http::HeaderMap,
    routing::{any, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    error::echo::{EchoError, JsonizeError},
    state::AppState,
};

pub fn route_echo() -> Router<AppState> {
    Router::new()
        .route("/", get(empty))
        .route("/", post(body))
        .route("/text/*rest", any(text))
        .route("/path/*rest", any(path))
        .route("/method", any(method))
        .route("/headers", any(headers))
        .route("/json", get(Jsonizer::dot_splitted_handler))
        .route("/json", post(json_body))
}

#[tracing::instrument]
pub async fn empty() -> &'static str {
    ""
}

#[tracing::instrument]
pub async fn body(body: Bytes) -> Bytes {
    body
}

#[tracing::instrument]
pub async fn text(OriginalUri(uri): OriginalUri) -> String {
    uri.to_string()
}

#[tracing::instrument]
pub async fn path(Path(rest): Path<String>) -> String {
    rest
}

#[tracing::instrument]
pub async fn method(request: Request) -> String {
    request.method().to_string()
}

#[tracing::instrument]
pub async fn headers(headers: HeaderMap) -> Json<Value> {
    Json(
        headers
            .into_iter()
            .map(|(name, value)| {
                let v = String::from_utf8_lossy(value.as_bytes()).to_string();
                if let Some(n) = name.as_ref().map(ToString::to_string) {
                    json!({n: v})
                } else {
                    json!(v)
                }
            })
            .collect(),
    )
}

#[tracing::instrument]
pub async fn json_body(body: Json<Value>) -> Json<Value> {
    body
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Jsonizer(pub Vec<(String, String)>);
impl Jsonizer {
    pub fn entry<'a, I: Iterator<Item = &'a str>>(
        value: &'a mut Value,
        mut path: I,
    ) -> Result<&'a mut Value, JsonizeError> {
        path.try_fold(value, |v, p| match v {
            Value::Object(o) => Ok(o.entry(p).or_insert(Value::Null)),
            Value::Array(a) => {
                let (idx, len) = (p.parse::<usize>()?, a.len());
                if idx >= len {
                    a.extend(vec![Value::Null; idx + 1 - len]);
                }
                Ok(&mut a[idx])
            }
            Value::Null => {
                if let Ok(idx) = p.parse::<usize>() {
                    let mut null = Value::Array(vec![Value::Null; idx + 1]);
                    std::mem::swap(v, &mut null);
                    Ok(v.as_array_mut().unwrap().get_mut(idx).unwrap())
                } else {
                    let mut null = Value::Object(Default::default());
                    std::mem::swap(v, &mut null);
                    Ok(v.as_object_mut().unwrap().entry(p.to_string()).or_insert(null))
                }
            }
            val => {
                let idx = p.parse::<usize>()?;
                let mut array = Value::Array(vec![Value::Null; idx + 1]);
                std::mem::swap(val, &mut array);
                *val.as_array_mut().unwrap().first_mut().unwrap() = array;
                Ok(&mut val[idx])
            }
        })
    }
    pub fn put(v: &mut Value, p: Value) {
        match v {
            Value::Null => *v = p,
            Value::Array(a) => {
                a.push(p);
            }
            _ => {
                let mut array = Value::Null;
                std::mem::swap(v, &mut array);
                *v = Value::Array(vec![array, p]);
            }
        }
    }
    pub fn parse(v: &str) -> Value {
        if let Ok(int) = v.parse::<i64>() {
            json!(int)
        } else if let Ok(float) = v.parse::<f64>() {
            json!(float)
        } else if let Ok(bool) = v.parse::<bool>() {
            json!(bool)
        } else if v == "null" {
            json!(null)
        } else {
            json!(v)
        }
    }
    pub fn dot_splitted(&self) -> Result<Value, JsonizeError> {
        let mut value = Value::Null;
        for (k, v) in &self.0 {
            Self::put(Self::entry(&mut value, k.split('.'))?, Self::parse(v));
        }
        Ok(value)
    }

    #[tracing::instrument]
    pub async fn dot_splitted_handler(Query(v): Query<Vec<(String, String)>>) -> Result<Json<Value>, EchoError> {
        Ok(Json(Self(v).dot_splitted()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::route::app_with;
    use crate::route::tests::{call_bytes, call_with_assert};

    use super::*;
    use axum::body::Body;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Method, Request, StatusCode};
    use mime::APPLICATION_JSON;

    #[tokio::test]
    async fn test_echo_empty() {
        let mut app = app_with(Default::default());

        let (status, body) = call_bytes(&mut app, Request::builder().uri("/echo/").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"");
    }

    #[tokio::test]
    async fn test_echo_body() {
        let mut app = app_with(Default::default());

        let (status, body) = call_bytes(
            &mut app,
            Request::builder().uri("/echo/").method(Method::POST).body(Body::from("hello world")).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"hello world");
    }

    #[tokio::test]
    async fn test_echo_text() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/echo/text/path?key=value").body(Body::empty()).unwrap())
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"/echo/text/path?key=value");
    }

    #[tokio::test]
    async fn test_echo_path() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/echo/path/query?key=value").body(Body::empty()).unwrap())
                .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"query");
    }

    #[tokio::test]
    async fn test_echo_method() {
        let mut app = app_with(Default::default());

        let (status, body) = call_bytes(
            &mut app,
            Request::builder().uri("/echo/method").method(Method::OPTIONS).body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"OPTIONS");
    }

    #[tokio::test]
    async fn test_echo_headers() {
        let mut app = app_with(Default::default());

        call_with_assert(
            &mut app,
            Request::builder()
                .uri("/echo/headers")
                .header("key1", "value1")
                .header("key2", "value2")
                .body(Body::empty())
                .unwrap(),
            StatusCode::OK,
            json!([{ "key1": "value1" }, { "key2": "value2" }]),
        )
        .await;
    }

    #[tokio::test]
    async fn test_jsonizer() {
        let j = Jsonizer(vec![]);
        assert_eq!(j.dot_splitted().unwrap(), Value::Null);

        let j = Jsonizer(vec![(String::from("key"), String::from("value"))]);
        assert_eq!(j.dot_splitted().unwrap(), json!({ "key": "value" }));

        let j = Jsonizer(vec![
            (String::from("key"), String::from("value1")),
            (String::from("key"), String::from("value2")),
        ]);
        assert_eq!(j.dot_splitted().unwrap(), json!({ "key": ["value1", "value2"] }));

        let j = Jsonizer(vec![(String::from("foo.bar.baz"), String::from("value"))]);
        assert_eq!(j.dot_splitted().unwrap(), json!({ "foo": { "bar": { "baz": "value" } } }));

        let j = Jsonizer(vec![
            (String::from("foo.bar.baz"), String::from("value1")),
            (String::from("foo.bar.baz"), String::from("value2")),
        ]);
        assert_eq!(j.dot_splitted().unwrap(), json!({ "foo": { "bar": { "baz": ["value1", "value2"] } } }));

        let j = Jsonizer(vec![(String::from("number.3.value"), String::from("three"))]);
        assert_eq!(j.dot_splitted().unwrap(), json!({ "number": [null, null, null, { "value": "three" }] }));

        let j = Jsonizer(vec![
            (String::from("number.3.value"), String::from("three")),
            (String::from("number.1.value"), String::from("one")),
        ]);
        assert_eq!(
            j.dot_splitted().unwrap(),
            json!({ "number": [null, { "value": "one" }, null, { "value": "three" }] })
        );

        let j = Jsonizer(vec![
            (String::from("hoge.fuga"), String::from("hogera")),
            (String::from("hoge.fuga.piyo"), String::from("hogehoge")),
        ]);
        assert!(j.dot_splitted().is_err()); // hoge.fuga will be [hogera, {piyo: hogehoge}], but in this case that is not hoge.fuga.piyo but hoge.fuga.1.piyo

        let j = Jsonizer(vec![
            (String::from("hoge.fuga"), String::from("hogera")),
            (String::from("hoge.fuga.1.piyo"), String::from("hogehoge")),
        ]);
        assert_eq!(j.dot_splitted().unwrap(), json!({ "hoge": { "fuga": ["hogera", { "piyo": "hogehoge" }] } }));
    }

    #[tokio::test]
    async fn test_echo_json() {
        let mut app = app_with(Default::default());

        call_with_assert(
            &mut app,
            Request::builder().uri("/echo/json").body(Body::empty()).unwrap(),
            StatusCode::OK,
            json!(null),
        )
        .await;

        call_with_assert(
            &mut app,
            Request::builder()
                .uri("/echo/json?key=value&a.foo=null&a.bar=true&a.baz=2.0&a.qux=three&a.quux=4&d.5=five&d.0=zero")
                .body(Body::empty())
                .unwrap(),
            StatusCode::OK,
            json!({
                "key": "value",
                "a": { "foo": null, "bar": true, "baz": 2.0, "qux": "three", "quux": 4 },
                "d": [ "zero", null, null, null, null, "five" ],
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn test_echo_json_post() {
        let mut app = app_with(Default::default());

        call_with_assert(
            &mut app,
            Request::builder()
                .uri("/echo/json")
                .method(Method::POST)
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{"key": "value"}"#))
                .unwrap(),
            StatusCode::OK,
            json!({ "key": "value" }),
        )
        .await;
    }
}