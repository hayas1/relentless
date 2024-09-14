#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    pub bind: String,
}
impl Default for Env {
    fn default() -> Self {
        Self { bind: "0.0.0.0:3000".to_string() }
    }
}
