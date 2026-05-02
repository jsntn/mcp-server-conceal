//! De-anonymization: reverse fake values back to original values in responses
//!
//! When the AI responds with fake PII values (e.g., "mike.wilson@techcorp.com"),
//! this module looks up the mapping database and restores the real values
//! (e.g., "john.smith@acme.com") before showing the response to the user.

use anyhow::Result;
use rusqlite::params;
use tracing::{debug, info};

use crate::mapping::MappingStore;

impl MappingStore {
    /// Get all fake→original mappings for de-anonymization.
    /// Returns a Vec of (fake_value, original_value_hash) pairs.
    /// Since we store hashes of originals, we need a separate reverse index.
    pub fn get_all_reverse_mappings(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT fake_value, original_value_hash FROM entity_mappings"
        )?;

        let mappings = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut results = Vec::new();
        for mapping in mappings {
            results.push(mapping?);
        }

        debug!("Loaded {} reverse mappings for de-anonymization", results.len());
        Ok(results)
    }

    /// Store a reverse mapping entry (fake_value → original_value in plaintext).
    /// This is stored in a separate table to support de-anonymization.
    pub fn store_reverse_mapping(&mut self, fake_value: &str, original_value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO reverse_mappings (fake_value, original_value)
             VALUES (?1, ?2)",
            params![fake_value, original_value],
        )?;
        debug!("Stored reverse mapping: {} -> [REDACTED]", fake_value);
        Ok(())
    }

    /// Look up the original value for a given fake value.
    pub fn reverse_lookup(&self, fake_value: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        let original: Option<String> = self.conn
            .query_row(
                "SELECT original_value FROM reverse_mappings WHERE fake_value = ?1",
                params![fake_value],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(ref val) = original {
            debug!("Reverse lookup hit for fake value: {}", fake_value);
            let _ = val; // avoid unused warning in release
        }
        Ok(original)
    }

    /// Get all reverse mappings as (fake_value, original_value) pairs for bulk replacement.
    pub fn get_all_reverse_entries(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT fake_value, original_value FROM reverse_mappings"
        )?;

        let mappings = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut results = Vec::new();
        for mapping in mappings {
            results.push(mapping?);
        }

        Ok(results)
    }

    /// Initialize the reverse_mappings table schema.
    /// Called during MappingStore initialization.
    pub fn initialize_reverse_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS reverse_mappings (
                fake_value TEXT PRIMARY KEY,
                original_value TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reverse_fake
             ON reverse_mappings(fake_value)",
            [],
        )?;

        Ok(())
    }
}

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
