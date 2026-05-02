# Integration: Wiring De-anonymization into proxy.rs

Three changes needed in `crates/mcp-server-conceal-core/src/proxy.rs`:

## 1. Add import at top

```rust
use crate::bootstrap::init_deanonymize;
use crate::deanonymize::deanonymize_text;
```

## 2. In `IntegratedProxy::new()`, after `MappingStore::new(...)`:

```rust
let mapping_store = MappingStore::new(config.config.mapping.clone())?;
init_deanonymize(&mapping_store)?;  // <-- ADD THIS LINE
```

## 3. In `create_anonymized_entities()`, after `mapping_store.store_mapping(&anonymized)?;`:

```rust
mapping_store.store_mapping(&anonymized)?;
mapping_store.store_reverse_mapping(&anonymized.fake_value, &anonymized.original_value)?;  // <-- ADD
```

Also add reverse mapping for the existing-mapping branch:
```rust
AnonymizedEntity {
    entity_type: entity.entity_type,
    original_value: entity.original_value.clone(),
    fake_value: existing_fake.clone(),
    mapping_id: format!("existing-{}", ...),
}
// ADD after this block:
// mapping_store.store_reverse_mapping(&existing_fake, &entity.original_value)?;
```

## 4. In `process_stdout_loop()`, change the response handling

Replace the call to `process_and_forward_line` for "response" direction with
de-anonymization instead of anonymization:

```rust
Ok(_) => {
    // De-anonymize: replace fake values with originals in AI responses
    let original_line = line.trim();
    match deanonymize_text(original_line, &mapping_store) {
        Ok(restored_line) => {
            if restored_line != original_line {
                info!("De-anonymized response");
            }
            our_stdout.write_all((restored_line + "\n").as_bytes()).await?;
            our_stdout.flush().await?;
        }
        Err(e) => {
            warn!("De-anonymization failed, forwarding as-is: {}", e);
            our_stdout.write_all(line.as_bytes()).await?;
            our_stdout.flush().await?;
        }
    }
}
```

## Summary

After these changes:
- Requests (stdin→child): PII detected → anonymized → fake values sent to AI
- Responses (child→stdout): fake values → de-anonymized → real values shown to user
