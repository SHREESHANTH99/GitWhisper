use crate::ai::AiBackend;
use crate::config::{AiConfig, AiProvider};

pub fn choose_backends(
    config: &AiConfig,
    prompt: &str,
    has_api_key: bool,
    offline_mode: bool,
) -> Vec<AiBackend> {
    match config.provider {
        AiProvider::Cloud => {
            if offline_mode || !has_api_key {
                Vec::new()
            } else {
                vec![AiBackend::Cloud]
            }
        }
        AiProvider::Local => vec![AiBackend::Local],
        AiProvider::Hybrid => {
            let cloud_ok = !offline_mode && has_api_key;
            let local_ok = true;

            if !cloud_ok && local_ok {
                return vec![AiBackend::Local];
            }

            if cloud_ok && !local_ok {
                return vec![AiBackend::Cloud];
            }

            if !cloud_ok && !local_ok {
                return Vec::new();
            }

            let prefer_local = prompt.chars().count() <= config.hybrid_max_prompt_chars;
            if prefer_local {
                vec![AiBackend::Local, AiBackend::Cloud]
            } else {
                vec![AiBackend::Cloud, AiBackend::Local]
            }
        }
    }
}
