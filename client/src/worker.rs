use reqwest::{Request, Url};
use tower::Service;

pub async fn run<S: Service<Request>>(mut service: S) -> Result<S::Response, S::Error> {
    let url = "http://localhost:3000";
    service
        .call(Request::new(reqwest::Method::GET, Url::parse(url).unwrap()))
        .await
}
