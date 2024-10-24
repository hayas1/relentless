#[tracing::instrument]
pub async fn root() -> String {
    "Hello World".to_string()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::route::{app_with, tests::call_bytes};

    use super::*;

    #[tokio::test]
    async fn test_root_function() {
        let res = root().await;
        assert_eq!(res, "Hello World");
    }

    #[tokio::test]
    async fn test_root() {
        let mut app = app_with(Default::default());

        let (status, body) = call_bytes(&mut app, Request::builder().uri("/").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"Hello World");
    }
}
