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

### Standalone MCP Server

Exposes privacy tools directly. You decide what to anonymize.

```bash
mcp-server-conceal --mode server --keep-database
```

**Arguments:**

| Arg | Description |
|-----|-------------|
| `--mode server` | Run as standalone MCP server (required) |
| `--keep-database` | Preserve mappings across restarts (recommended) |
| `--config <path>` | Custom config file path (optional, auto-created if omitted) |
| `--log-level <level>` | Log verbosity: error, warn, info, debug, trace (default: info) |

**Tools exposed:**

| Tool | Description |
|------|-------------|
| `privacy_anonymize(text)` | Detect and replace PII with consistent fake values |
| `privacy_deanonymize(text)` | Restore original values from previously anonymized text |
| `privacy_status` | Show mapping count and entity type breakdown |

**MCP client config:**

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

### Proxy Mode

Wraps another MCP server. Automatically anonymizes PII in requests sent to the target server and de-anonymizes responses coming back. No manual tool calls needed — all traffic is processed transparently.

```bash
mcp-server-conceal \
  --target-command python3 \
  --target-args "my-mcp-server.py" \
  --keep-database
```

**Arguments:**

| Arg | Description |
|-----|-------------|
| `--target-command <cmd>` | Command to launch the target MCP server (required) |
| `--target-args <args>` | Arguments for the target server (space-separated, supports quotes) |
| `--target-env <KEY=VALUE>` | Environment variables for the target (repeatable) |
| `--target-cwd <path>` | Working directory for the target server |
| `--keep-database` | Preserve mappings across restarts |
| `--config <path>` | Custom config file path |
| `--log-level <level>` | Log verbosity (default: info) |

**How it works:**

```
MCP Client ←stdio→ mcp-server-conceal ←stdio→ Target MCP Server
                         │
                    Anonymize requests (PII → fake)
                    De-anonymize responses (fake → real)
```

**Example — wrap a database MCP server for Claude Desktop:**

```json
{
  "mcpServers": {
    "database": {
      "command": "mcp-server-conceal",
      "args": [
        "--target-command", "python3",
        "--target-args", "database-server.py --host localhost",
        "--keep-database"
      ],
      "env": {
        "DATABASE_URL": "postgresql://localhost/mydb"
      }
    }
  }
}
```

In this setup, any PII in tool responses from the database server is anonymized before reaching the AI, and any fake values the AI uses in subsequent requests are de-anonymized before reaching the database server.

## Quick Start

### Option A: NER Detection (Recommended)

1. Download the ONNX model (~412MB, one-time):

```bash
mkdir -p ~/.local/share/mcp-server-conceal/ner
cd ~/.local/share/mcp-server-conceal/ner
curl -L -o model.onnx "https://huggingface.co/dslim/bert-base-NER/resolve/main/onnx/model.onnx"
curl -L -o tokenizer.json "https://huggingface.co/dslim/bert-base-NER/resolve/main/onnx/tokenizer.json"
```

2. Add to `~/.config/mcp-server-conceal/mcp-server-conceal.toml`:

```toml
[detection]
mode = "regex_ner"

[ner]
model_path = "/home/YOUR_USER/.local/share/mcp-server-conceal/ner/model.onnx"
tokenizer_path = "/home/YOUR_USER/.local/share/mcp-server-conceal/ner/tokenizer.json"
labels = ["O", "B-MISC", "I-MISC", "B-PER", "I-PER", "B-ORG", "I-ORG", "B-LOC", "I-LOC"]
```

3. Run: `mcp-server-conceal --mode server --keep-database`

### Option B: LLM Detection (Legacy)

1. Install Ollama: [ollama.ai](https://ollama.ai)
2. Pull model: `ollama pull qwen2.5:1.5b-instruct-q4_K_M`
3. Verify: `curl http://localhost:11434/api/version`
4. Run: `mcp-server-conceal --mode server --keep-database`

Config is auto-created at `~/.config/mcp-server-conceal/mcp-server-conceal.toml`.

## Detection Backends

### NER (Named Entity Recognition) — Recommended

This branch embeds a BERT-based NER model directly in the binary using [tract](https://github.com/sonos/tract) (pure Rust ONNX runtime). No Python, no external services — single self-contained binary.

| | NER (embedded ONNX) | LLM (Ollama) |
|---|---|---|
| **Speed** | ~1s per call | 5-60s |
| **Accuracy** | High — catches names, locations, orgs | Misses contextual PII |
| **Consistency** | Deterministic | Non-deterministic |
| **Resource usage** | ~500MB (model in memory) | ~1-2GB RAM |
| **Dependencies** | None (single binary + model file) | Ollama running |
| **False positives** | Very low | Can hallucinate |

The default model (`dslim/bert-base-NER`) detects: persons, locations, organizations, and miscellaneous entities.

#### NER Setup

1. Download the ONNX model and tokenizer from Hugging Face (one-time, ~412MB):

```bash
mkdir -p ~/.local/share/mcp-server-conceal/ner
cd ~/.local/share/mcp-server-conceal/ner
curl -L -o model.onnx "https://huggingface.co/dslim/bert-base-NER/resolve/main/onnx/model.onnx"
curl -L -o tokenizer.json "https://huggingface.co/dslim/bert-base-NER/resolve/main/onnx/tokenizer.json"
```

2. Configure in `mcp-server-conceal.toml`:

```toml
[detection]
mode = "regex_ner"

[ner]
model_path = "/home/YOUR_USER/.local/share/mcp-server-conceal/ner/model.onnx"
tokenizer_path = "/home/YOUR_USER/.local/share/mcp-server-conceal/ner/tokenizer.json"
# Labels must match the model's output. Each word gets one label:
#   O      = not an entity (ignored)
#   B-PER  = beginning of a person name     → replaced with fake name
#   I-PER  = continuation of a person name
#   B-ORG  = beginning of an organization   → redacted
#   I-ORG  = continuation of an organization
#   B-LOC  = beginning of a location        → redacted
#   I-LOC  = continuation of a location
#   B-MISC = beginning of a misc entity     → redacted
#   I-MISC = continuation of a misc entity
# "B-" = first word, "I-" = following words of the same entity.
labels = ["O", "B-MISC", "I-MISC", "B-PER", "I-PER", "B-ORG", "I-ORG", "B-LOC", "I-LOC"]
```

Any ONNX token-classification model with BIO tagging can be used — just update the model path and labels list.

### LLM (Ollama) — Legacy

The LLM approach uses a generative model to detect PII via prompting. Still supported but not recommended for production use.

| Model | Size | Best for |
|-------|------|----------|
| `qwen2.5:1.5b-instruct-q4_K_M` | ~1GB | Low storage, good for structured PII |
| `qwen2.5:3b-instruct-q4_K_M` | ~2GB | Better name/address detection |
| `llama3.2:3b` | ~2GB | Well-rounded |

## Detection Modes

| Mode | Latency | Accuracy | Configure |
|------|---------|----------|-----------|
| `regex_ner` | ~1s | Best | Regex first, NER for remainder |
| `regex_llm` | 5-60s | Good | Regex first, LLM for remainder |
| `regex` | <10ms | Good for structured PII | Pattern matching only |
| `llm` | 5-60s | Moderate | AI-only detection (legacy) |

## De-anonymization

The mapping database stores fake→real pairs. Consistent mapping ensures the same real PII always maps to the same fake value across sessions (when using `--keep-database`).

- **Forward table:** stores hash(original) → fake (for consistency)
- **Reverse table:** stores fake → original (for de-anonymization)

## Building from Source

```bash
git clone https://github.com/jsntn/mcp-server-conceal
cd mcp-server-conceal
cargo build --release
```

Requires Rust 1.85+. Binary: `target/release/mcp-server-conceal`

## Configuration

See `mcp-server-conceal.example.toml` for all options, including NER model setup and LLM configuration.

Custom LLM prompts can be placed at `~/.local/share/mcp-server-conceal/prompts/default.md`.

## Security

- **Reverse mappings** contain plaintext originals. Protect `~/.local/share/mcp-server-conceal/`.
- **NER model runs locally** — embedded in the binary via tract. No data leaves your machine.
- **LLM runs locally** via Ollama — no data leaves your machine.
- **Forward mappings** store hashes of originals (not plaintext).

## License

MIT License - see LICENSE file for details.

## Credits

Originally created by [Gianluca Brigandi](https://github.com/gbrigandi/mcp-server-conceal). This fork adds standalone MCP server mode, de-anonymization, and a smaller default LLM model.
