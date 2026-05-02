# MCP Conceal

An MCP privacy tool that pseudo-anonymizes PII and de-anonymizes responses. Runs as a **standalone MCP server** or as a **proxy** in front of another MCP server.

```mermaid
sequenceDiagram
    participant U as You
    participant P as MCP Conceal
    participant AI as AI Provider
    
    U->>P: privacy_anonymize("email john@real.com")
    P->>P: Detect PII → Replace with fake
    P->>U: "email mike@fake.org"
    Note over U,AI: You send anonymized text to AI
    AI->>U: "I emailed mike@fake.org"
    U->>P: privacy_deanonymize("I emailed mike@fake.org")
    P->>P: Reverse lookup → Restore real
    P->>U: "I emailed john@real.com"
```

## Two Modes

### Standalone MCP Server (recommended for MCP clients)

Exposes privacy tools directly:

```bash
mcp-server-conceal --mode server --keep-database
```

**Tools exposed:**
- `privacy_anonymize(text)` — detect and replace PII with fake values
- `privacy_deanonymize(text)` — restore original values from fakes
- `privacy_status` — show mapping statistics

**MCP client config (e.g., kiro-cli, Claude Desktop):**

```json
{
  "mcpServers": {
    "conceal": {
      "command": "mcp-server-conceal",
      "args": ["--mode", "server", "--keep-database"]
    }
  }
}
```

### Proxy Mode (wraps another MCP server)

Transparently anonymizes/de-anonymizes all traffic:

```bash
mcp-server-conceal \
  --target-command python3 \
  --target-args "my-mcp-server.py"
```

## Quick Start

### Prerequisites

1. Install Ollama: [ollama.ai](https://ollama.ai)
2. Pull model: `ollama pull qwen2.5:1.5b-instruct-q4_K_M`
3. Verify: `curl http://localhost:11434/api/version`

Config is auto-created at `~/.config/mcp-server-conceal/mcp-server-conceal.toml`.

## LLM Model Selection

The LLM detects PII that regex misses (names, addresses, contextual data). An **instruct model** is required — it follows structured prompts to return PII entities as JSON.

| Model | Size | Best for |
|-------|------|----------|
| `qwen2.5:1.5b-instruct-q4_K_M` | ~1GB | Low storage, good for structured PII |
| `qwen2.5:3b-instruct-q4_K_M` | ~2GB | Better name/address detection |
| `llama3.2:3b` | ~2GB | Well-rounded |

**When the LLM matters:** Regex catches emails, phones, SSNs, credit cards, and IPs instantly. The LLM only adds value for **names and unstructured contextual PII**.

## Detection Modes

| Mode | Latency | Accuracy | Configure |
|------|---------|----------|-----------|
| `regex_llm` (default) | 5-60s | High | Regex first, LLM for remainder |
| `regex` | <10ms | Good for structured PII | Pattern matching only |
| `llm` | 5-60s | Best for unstructured text | AI-only detection |

## De-anonymization

The mapping database stores fake→real pairs. When you call `privacy_deanonymize`, it replaces fake values with originals. Consistent mapping ensures the same real PII always maps to the same fake.

## Building from Source

```bash
git clone https://github.com/jsntn/mcp-server-conceal
cd mcp-server-conceal
cargo build --release
```

Requires Rust 1.85+. Binary: `target/release/mcp-server-conceal`

## Configuration

See `mcp-server-conceal.example.toml` for all options.

## Security

- **Reverse mappings** contain plaintext originals. Protect `~/.local/share/mcp-server-conceal/`.
- **LLM runs locally** via Ollama — no data leaves your machine.
- **Forward mappings** store hashes of originals (not plaintext).

## License

MIT License - see LICENSE file for details.

## Credits

Originally created by [Gianluca Brigandi](https://github.com/gbrigandi/mcp-server-conceal). This fork adds standalone MCP server mode, de-anonymization, and a smaller default LLM model.
