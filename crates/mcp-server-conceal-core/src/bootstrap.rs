//! Bootstrap: ensures reverse_mappings table exists.
//! Called from proxy startup before processing begins.

use anyhow::Result;
use crate::mapping::MappingStore;

/// Initialize all de-anonymization infrastructure.
/// Call this once at proxy startup after MappingStore is created.
pub fn init_deanonymize(mapping_store: &MappingStore) -> Result<()> {
    mapping_store.initialize_reverse_schema()?;
    Ok(())
}
