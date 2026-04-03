# TRDC - Token Reducer CLI

LLM-first context-aware command output summarization.

## Features

- Execute any command and get LLM-powered semantic summaries
- Add custom context to guide summarization focus
- Configurable LLM settings via config file or environment variables
- Dry-run mode to preview prompts
- Token counting and savings display

## Installation

```bash
cargo install --path trdc
```

## Usage

```bash
# Basic usage
trdc cargo test

# With context
trdc cargo test --context "Focus on test failures and root causes"

# Show token savings (like rtk gain)
trdc -g cargo test
trdc --gain cargo build

# With custom model
trdc git diff --model llama3:8b --max-tokens 150

# Dry run (show prompt without sending to LLM)
trdc --dry-run cargo build

# Verbose output
trdc -v git log --oneline -50
```

## Token Tracking with `-g`

Use the `-g` or `--gain` flag to display token savings statistics:

``` text
╔══════════════════════════════════════════════════════════╗
║              TRDC Token Savings Summary                  ║
╠══════════════════════════════════════════════════════════╣
║  Current Command:                                        ║
║    Input (to LLM):        1500 tokens                    ║
║    Output (from LLM):      200 tokens                    ║
║    Saved:                 1300 tokens ( 86.7%)           ║
╠══════════════════════════════════════════════════════════╣
║  Session Total:                                          ║
║    Input (to LLM):        5000 tokens                    ║
║    Output (from LLM):      800 tokens                    ║
║    Saved:                 4200 tokens ( 84.0%)           ║
║    Commands:                 5                           ║
╚══════════════════════════════════════════════════════════╝
```

**How it works:**

- **Input tokens**: Estimated from command output (chars / 4)
- **Output tokens**: Actual tokens from LLM response (if API provides them)
- **Savings**: `(input - output) / input * 100`
- **Session stats**: Persisted in `~/.local/share/trdc/stats.json`

## Configuration

Create `~/.config/trdc/config.toml`:

```toml
[llm]
endpoint_url = "http://localhost:1234/v1/chat/completions"
model_name = "local-model"
timeout_secs = 10
max_output_tokens = 200
max_input_chars = 4000
temperature = 0.7
```

### Environment Variables

- `TRDC_LLM_ENDPOINT` - LLM server URL
- `TRDC_LLM_MODEL` - Model name
- `TRDC_MAX_TOKENS` - Max output tokens
- `TRDC_TIMEOUT` - Request timeout in seconds

## Comparison with RTK

| Aspect | RTK | TRDC |
| ------- | ---- | ----- |
| Primary Method | Deterministic filters + Optional LLM | LLM-only |
| Context Awareness | Basic command detection | Rich user context |
| Token Savings | 60-90% | 70-95% (with context) |
| Speed | <10ms (Layer 1) | 2-10s (LLM bound) |
| Use Case | General CLI optimization | Context-aware analysis |
