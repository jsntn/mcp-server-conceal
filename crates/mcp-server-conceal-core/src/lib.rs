use std::sync::atomic::{AtomicBool, Ordering};

static INFO_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_info_mode(enabled: bool) {
    INFO_MODE.store(enabled, Ordering::Relaxed);
}

pub fn info_mode() -> bool {
    INFO_MODE.load(Ordering::Relaxed)
}

pub mod proxy;
pub mod bootstrap;
pub mod config;
pub mod detection;
pub mod deanonymize;
pub mod faker;
pub mod mapping;
pub mod ner;
pub mod ollama;
pub mod prompt_loader;
pub mod server;

#[cfg(test)]
pub mod integration_tests;

pub use proxy::{IntegratedProxy, IntegratedProxyConfig};
pub use bootstrap::init_deanonymize;
pub use config::{Config, DetectionConfig, FakerConfig, MappingConfig, LlmConfig, NerConfig, DetectedEntity, AnonymizedEntity};
pub use detection::RegexDetectionEngine;
pub use deanonymize::deanonymize_text;
pub use faker::FakerEngine;
pub use mapping::{MappingStore, EntityMapping, LlmCacheEntry, MappingStatistics};
pub use ner::NerEngine;
pub use ollama::{OllamaClient, OllamaConfig, LlmResponse, LlmDetectedEntity};
pub use prompt_loader::PromptLoader;
