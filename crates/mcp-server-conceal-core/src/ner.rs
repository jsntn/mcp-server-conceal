//! Embedded NER engine using tract (pure Rust ONNX runtime) for token-classification models.

use crate::config::{DetectedEntity, NerConfig};
use anyhow::{Context, Result};
use ndarray::Array2;
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;
use tracing::{debug, info};

type Model = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

pub struct NerEngine {
    model: Model,
    tokenizer: Tokenizer,
    labels: Vec<String>,
}

impl NerEngine {
    pub fn new(config: &NerConfig) -> Result<Self> {
        info!("Loading NER model from: {}", config.model_path);

        let model = tract_onnx::onnx()
            .model_for_path(&config.model_path)
            .context("Failed to load ONNX model")?
            .with_input_fact(0, i64::fact([1, 128]).into())?  // input_ids
            .with_input_fact(1, i64::fact([1, 128]).into())?  // attention_mask
            .with_input_fact(2, i64::fact([1, 128]).into())?  // token_type_ids
            .into_optimized()?
            .into_runnable()?;

        let tokenizer = Tokenizer::from_file(&config.tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        info!("NER engine loaded ({} labels)", config.labels.len());
        Ok(Self { model, tokenizer, labels: config.labels.clone() })
    }

    pub fn detect_entities(&self, text: &str) -> Result<Vec<DetectedEntity>> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let ids = encoding.get_ids();
        let attention = encoding.get_attention_mask();
        let len = ids.len();
        let max_len = 128;

        // Pad to max_len
        let mut padded_ids = vec![0i64; max_len];
        let mut padded_attn = vec![0i64; max_len];
        let actual_len = len.min(max_len);
        for i in 0..actual_len {
            padded_ids[i] = ids[i] as i64;
            padded_attn[i] = attention[i] as i64;
        }

        let input_ids = Array2::from_shape_vec((1, max_len), padded_ids)?;
        let attention_mask = Array2::from_shape_vec((1, max_len), padded_attn)?;
        let token_type_ids = Array2::<i64>::zeros((1, max_len));

        let result = self.model.run(tvec![
            input_ids.into_tvalue(),
            attention_mask.into_tvalue(),
            token_type_ids.into_tvalue(),
        ])?;

        let logits = result[0].to_array_view::<f32>()?;
        let num_labels = self.labels.len();
        let offsets = encoding.get_offsets();

        let mut entities = Vec::new();
        let mut current_entity: Option<(String, usize, usize, f32)> = None;

        for i in 0..actual_len {
            let (offset_start, offset_end) = offsets[i];

            // Skip special tokens
            if offset_start == 0 && offset_end == 0 && i > 0 {
                if let Some((etype, start, end, score)) = current_entity.take() {
                    push_entity(&mut entities, text, &etype, start, end, score);
                }
                continue;
            }

            // Find argmax
            let mut max_idx = 0;
            let mut max_val: f32 = f32::NEG_INFINITY;
            for j in 0..num_labels {
                let v = logits[[0, i, j]];
                if v > max_val {
                    max_val = v;
                    max_idx = j;
                }
            }

            let label = self.labels.get(max_idx).map(|s| s.as_str()).unwrap_or("O");

            if let Some(entity_type) = label.strip_prefix("B-") {
                if let Some((etype, start, end, score)) = current_entity.take() {
                    push_entity(&mut entities, text, &etype, start, end, score);
                }
                current_entity = Some((entity_type.to_string(), offset_start, offset_end, max_val));
            } else if let Some(entity_type) = label.strip_prefix("I-") {
                if let Some((ref etype, start, _, ref mut score)) = current_entity {
                    if etype == entity_type {
                        current_entity = Some((entity_type.to_string(), start, offset_end, score.max(max_val)));
                    } else {
                        let (etype, start, end, score) = current_entity.take().unwrap();
                        push_entity(&mut entities, text, &etype, start, end, score);
                        current_entity = Some((entity_type.to_string(), offset_start, offset_end, max_val));
                    }
                } else {
                    current_entity = Some((entity_type.to_string(), offset_start, offset_end, max_val));
                }
            } else {
                if let Some((etype, start, end, score)) = current_entity.take() {
                    push_entity(&mut entities, text, &etype, start, end, score);
                }
            }
        }

        if let Some((etype, start, end, score)) = current_entity.take() {
            push_entity(&mut entities, text, &etype, start, end, score);
        }

        debug!("NER detected {} entities", entities.len());
        Ok(entities)
    }
}

fn push_entity(entities: &mut Vec<DetectedEntity>, text: &str, entity_type: &str, start: usize, end: usize, score: f32) {
    let value = &text[start..end];
    if value.trim().is_empty() {
        return;
    }
    entities.push(DetectedEntity {
        entity_type: normalize_label(entity_type),
        original_value: value.to_string(),
        start,
        end,
        confidence: score as f64,
    });
}

fn normalize_label(label: &str) -> String {
    match label {
        "PER" | "PERSON" | "person" => "person_name".to_string(),
        "LOC" | "LOCATION" | "location" | "ADDRESS" | "address" => "address".to_string(),
        "ORG" | "ORGANIZATION" | "organization" => "organization".to_string(),
        "DATE" | "DATE_OF_BIRTH" | "date_of_birth" => "date_of_birth".to_string(),
        other => other.to_lowercase(),
    }
}
