#[tracing::instrument(ret)]
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

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(&res.body()[..], b"Hello World");
    }
}
