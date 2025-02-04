#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Env {
    pub listen: String,
    pub port: String,
}
impl Default for Env {
    fn default() -> Self {
        Self { listen: "127.0.0.1".to_string(), port: "8000".to_string() }
    }
}
impl Env {
    pub fn environment() -> Self {
        let default = Self::default();
        Self {
            listen: std::env::var("LISTEN").unwrap_or(default.listen),
            port: std::env::var("PORT").unwrap_or(default.port),
        }
    }

    pub fn bind(&self) -> String {
        format!("{}:{}", self.listen, self.port)
    }
}
