#[tracing::instrument]
pub async fn root() -> String {
    "Hello World".to_string()
}
