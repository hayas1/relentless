#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Env {
    pub listen: String,
    pub port: String,
}
impl Default for Env {
    fn default() -> Self {
        Self { listen: "0.0.0.0".to_string(), port: "50051".to_string() }
    }
}
impl Env {
    pub fn environment(default: Self) -> Self {
        Self {
            listen: std::env::var("LISTEN").unwrap_or(default.listen),
            port: std::env::var("PORT").unwrap_or(default.port),
        }
    }

    pub fn bind(&self) -> String {
        format!("{}:{}", self.listen, self.port)
    }
}
