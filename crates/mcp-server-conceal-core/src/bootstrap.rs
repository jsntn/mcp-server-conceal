//! Bootstrap: ensures reverse_mappings table exists.

use anyhow::Result;
use crate::mapping::MappingStore;

/// Initialize de-anonymization infrastructure. Call once at startup.
pub fn init_deanonymize(mapping_store: &MappingStore) -> Result<()> {
    mapping_store.initialize_reverse_schema()
}
