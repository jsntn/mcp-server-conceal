//! De-anonymization: reverse fake values back to original values in responses

use anyhow::Result;
use tracing::info;

use crate::mapping::MappingStore;

/// De-anonymize a text string by replacing all known fake values with originals.
pub fn deanonymize_text(text: &str, mapping_store: &MappingStore) -> Result<String> {
    let reverse_entries = mapping_store.get_all_reverse_entries()?;

    if reverse_entries.is_empty() {
        return Ok(text.to_string());
    }

    let mut result = text.to_string();
    let mut replacements = 0;

    // Sort by length descending to avoid partial replacements
    let mut sorted_entries = reverse_entries;
    sorted_entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (fake_value, original_value) in &sorted_entries {
        if result.contains(fake_value.as_str()) {
            result = result.replace(fake_value.as_str(), original_value.as_str());
            replacements += 1;
        }
    }

    if replacements > 0 {
        info!("De-anonymized {} PII values in response", replacements);
    }

    Ok(result)
}
