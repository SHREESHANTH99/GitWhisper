pub mod cloud_gemini;
pub mod context_optimizer;
pub mod local_ollama;
pub mod model_selector;
pub mod reasoning_chain;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBackend {
    Cloud,
    Local,
}

impl AiBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
        }
    }
}
