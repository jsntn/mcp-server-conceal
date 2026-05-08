//! Standalone MCP server mode — exposes privacy tools via JSON-RPC over stdio.

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

use crate::config::{Config, DetectedEntity, DetectionMode};
use crate::bootstrap::init_deanonymize;
use crate::deanonymize::deanonymize_text;
use crate::detection::RegexDetectionEngine;
use crate::faker::FakerEngine;
use crate::mapping::MappingStore;
use crate::ner::NerEngine;
use crate::ollama::{OllamaClient, OllamaConfig};

pub struct McpServer {
    detection_engine: RegexDetectionEngine,
    faker_engine: FakerEngine,
    mapping_store: MappingStore,
    ollama_client: OllamaClient,
    ner_client: Option<NerEngine>,
    detection_mode: DetectionMode,
    model_name: String,
}

impl McpServer {
    pub fn new(config: Config, ollama_config: OllamaConfig) -> Result<Self> {
        let detection_engine = RegexDetectionEngine::new(&config.detection)?;
        let faker_engine = FakerEngine::new(&config.faker);
        let mapping_store = MappingStore::new(config.mapping.clone())?;
        init_deanonymize(&mapping_store)?;
        let ollama_client = OllamaClient::new(ollama_config.clone(), config.llm.as_ref().and_then(|l| l.prompt_template.as_ref()))?;
        let ner_client = config.ner.as_ref().map(|c| NerEngine::new(c)).transpose()?;
        let detection_mode = config.detection.mode.clone();
        let model_name = ollama_config.model.clone();

        Ok(Self { detection_engine, faker_engine, mapping_store, ollama_client, ner_client, detection_mode, model_name })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting MCP Conceal server (standalone mode)");
        let mut reader = BufReader::new(stdin());
        let mut out = stdout();
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() { continue; }
                    match serde_json::from_str::<Value>(trimmed) {
                        Ok(msg) => {
                            let resp = self.handle_message(&msg).await;
                            let out_str = serde_json::to_string(&resp)? + "\n";
                            out.write_all(out_str.as_bytes()).await?;
                            out.flush().await?;
                        }
                        Err(e) => {
                            error!("Invalid JSON: {}", e);
                            let err = json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}});
                            out.write_all((serde_json::to_string(&err)? + "\n").as_bytes()).await?;
                            out.flush().await?;
                        }
                    }
                }
                Err(e) => { error!("Read error: {}", e); break; }
            }
        }
        Ok(())
    }

    async fn handle_message(&mut self, msg: &Value) -> Value {
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");

        match method {
            "initialize" => json!({
                "jsonrpc": "2.0", "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "mcp-server-conceal", "version": "0.2.0"}
                }
            }),
            "notifications/initialized" => return Value::Null,
            "tools/list" => json!({
                "jsonrpc": "2.0", "id": id,
                "result": {"tools": [
                    {
                        "name": "privacy_anonymize",
                        "description": "Anonymize PII in text. Returns text with fake values replacing real PII.",
                        "inputSchema": {"type": "object", "properties": {"text": {"type": "string", "description": "Text containing PII to anonymize"}}, "required": ["text"]}
                    },
                    {
                        "name": "privacy_deanonymize",
                        "description": "Restore original PII values in text that was previously anonymized.",
                        "inputSchema": {"type": "object", "properties": {"text": {"type": "string", "description": "Text containing fake PII to restore"}}, "required": ["text"]}
                    },
                    {
                        "name": "privacy_status",
                        "description": "Show current mapping statistics.",
                        "inputSchema": {"type": "object", "properties": {}}
                    }
                ]}
            }),
            "tools/call" => {
                let params = msg.get("params").cloned().unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let result = self.handle_tool_call(tool_name, &args).await;
                json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": result}]}})
            }
            _ => json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("Unknown method: {}", method)}})
        }
    }

    async fn handle_tool_call(&mut self, tool: &str, args: &Value) -> String {
        match tool {
            "privacy_anonymize" => {
                let text = args.get("text").and_then(|t| t.as_str()).unwrap_or("");
                match self.anonymize(text).await {
                    Ok(result) => result,
                    Err(e) => format!("Error: {}", e),
                }
            }
            "privacy_deanonymize" => {
                let text = args.get("text").and_then(|t| t.as_str()).unwrap_or("");
                match deanonymize_text(text, &self.mapping_store) {
                    Ok(result) => result,
                    Err(e) => format!("Error: {}", e),
                }
            }
            "privacy_status" => {
                match self.mapping_store.get_statistics() {
                    Ok(stats) => format!("Mappings: {}, Cache: {}, Types: {:?}", stats.total_mappings, stats.total_cache_entries, stats.mappings_by_type),
                    Err(e) => format!("Error: {}", e),
                }
            }
            _ => format!("Unknown tool: {}", tool),
        }
    }

    /// Detect known original values from the mapping database that appear in the text.
    fn detect_known_entities(&self, text: &str) -> Vec<DetectedEntity> {
        let mut entities = Vec::new();
        let reverse_entries = match self.mapping_store.get_all_reverse_entries() {
            Ok(entries) => entries,
            Err(_) => return entities,
        };

        for (_fake_value, original_value) in &reverse_entries {
            // Search for all occurrences of this known original value in the text
            let mut start = 0;
            while let Some(pos) = text[start..].find(original_value.as_str()) {
                let abs_start = start + pos;
                let abs_end = abs_start + original_value.len();
                entities.push(DetectedEntity {
                    entity_type: "person_name".to_string(),
                    original_value: original_value.clone(),
                    start: abs_start,
                    end: abs_end,
                    confidence: 1.0,
                });
                start = abs_end;
            }
        }

        debug!("Detected {} known entities from mapping database", entities.len());
        entities
    }

    async fn anonymize(&mut self, text: &str) -> Result<String> {
        let mut entities = match &self.detection_mode {
            DetectionMode::Regex => self.detection_engine.detect_in_text(text),
            DetectionMode::Llm => {
                if let Some(cached) = self.mapping_store.get_llm_cache(text, &self.model_name)? {
                    cached
                } else if self.ollama_client.health_check().await.unwrap_or(false) {
                    let ents = self.ollama_client.extract_entities(text).await.unwrap_or_default();
                    self.mapping_store.store_llm_cache(text, &ents, &self.model_name)?;
                    ents
                } else {
                    vec![]
                }
            }
            DetectionMode::RegexLlm => {
                let mut regex_ents = self.detection_engine.detect_in_text(text);
                if self.ollama_client.health_check().await.unwrap_or(false) {
                    if let Some(cached) = self.mapping_store.get_llm_cache(text, &self.model_name)? {
                        regex_ents.extend(cached);
                    } else {
                        let llm_ents = self.ollama_client.extract_entities(text).await.unwrap_or_default();
                        self.mapping_store.store_llm_cache(text, &llm_ents, &self.model_name)?;
                        regex_ents.extend(llm_ents);
                    }
                }
                regex_ents
            }
            DetectionMode::RegexNer => {
                let mut regex_ents = self.detection_engine.detect_in_text(text);
                if let Some(ref ner) = self.ner_client {
                    match ner.detect_entities(text) {
                        Ok(ner_ents) => {
                            debug!("NER returned {} entities", ner_ents.len());
                            regex_ents.extend(ner_ents);
                        }
                        Err(e) => {
                            error!("NER detection failed: {}", e);
                        }
                    }
                }
                regex_ents
            }
        };

        // Also detect any known original values from the mapping database.
        // This ensures previously seen PII is always caught, even if the LLM misses it.
        let known_ents = self.detect_known_entities(text);
        for known in known_ents {
            if !entities.iter().any(|e| e.original_value == known.original_value && e.start == known.start) {
                entities.push(known);
            }
        }

        if entities.is_empty() {
            return Ok(text.to_string());
        }

        // Sort by start position descending so replacements don't shift indices
        entities.sort_by(|a, b| b.start.cmp(&a.start));
        // Deduplicate overlapping entities (keep higher confidence)
        entities.dedup_by(|a, b| {
            if a.start >= b.start && a.start < b.end {
                if a.confidence > b.confidence {
                    std::mem::swap(a, b);
                }
                true
            } else {
                false
            }
        });

        let mut result = text.to_string();
        for entity in &entities {
            let fake = if let Some(existing) = self.mapping_store.get_mapping(&entity.entity_type, &entity.original_value)? {
                self.mapping_store.store_reverse_mapping(&existing, &entity.original_value)?;
                existing
            } else {
                let anonymized = self.faker_engine.anonymize_entity(entity)?;
                self.mapping_store.store_mapping(&anonymized)?;
                self.mapping_store.store_reverse_mapping(&anonymized.fake_value, &anonymized.original_value)?;
                anonymized.fake_value
            };
            if crate::info_mode() {
                eprintln!("[CONCEAL] Anonymized: {} \"{}\" → \"{}\"", entity.entity_type, entity.original_value, fake);
            }
            result = result.replacen(&entity.original_value, &fake, 1);
        }

        info!("Anonymized {} entities", entities.len());
        Ok(result)
    }
}
