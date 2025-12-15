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

    use crate::app::{tests::call_bytes, AppRouter};

    use super::*;

    #[tokio::test]
    async fn test_root_function() {
        let res = root().await;
        assert_eq!(res, "Hello World");
    }

    #[tokio::test]
    async fn test_root() {
        let mut service = AppRouter::default().service();

        let (status, body) = call_bytes(&mut service, Request::builder().uri("/").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"Hello World");
    }
}
