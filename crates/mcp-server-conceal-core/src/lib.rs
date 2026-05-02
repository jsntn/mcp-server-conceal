pub mod proxy;
pub mod bootstrap;
pub mod config;
pub mod detection;
pub mod deanonymize;
pub mod faker;
pub mod mapping;
pub mod ollama;
pub mod prompt_loader;
pub mod server;

#[cfg(test)]
pub mod integration_tests;

pub use proxy::{IntegratedProxy, IntegratedProxyConfig};
pub use bootstrap::init_deanonymize;
pub use config::{Config, DetectionConfig, FakerConfig, MappingConfig, LlmConfig, DetectedEntity, AnonymizedEntity};
pub use detection::RegexDetectionEngine;
pub use deanonymize::deanonymize_text;
pub use faker::FakerEngine;
pub use mapping::{MappingStore, EntityMapping, LlmCacheEntry, MappingStatistics};
pub use ollama::{OllamaClient, OllamaConfig, LlmResponse, LlmDetectedEntity};
pub use prompt_loader::PromptLoader;
