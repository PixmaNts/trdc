# TRDC - Token Reducer CLI

LLM-first context-aware command output summarization (v2.0).

## Features

- Execute any command and get LLM-powered semantic summaries
- Add custom context to guide summarization focus
- Configurable LLM settings via config file or environment variables
- Dry-run mode to preview prompts
- Token counting and savings display
- Streaming output for real-time summary feedback
- Exit code propagation from subprocess
- Pre-filtering to reduce token count before LLM processing
- Stdin support for piping output from other commands
- Command history tracking with `--history` flag

## Installation

```bash
cargo install --path trdc
```

## Usage

```bash
# Basic usage
trdc cargo test

# With context
trdc --context "Focus on test failures and root causes" cargo test

# Show token savings (like rtk gain)
trdc --gain cargo test

# With custom model
trdc git diff --model llama3:8b --max-tokens 150

# Dry run (show prompt without sending to LLM)
trdc --dry-run cargo build

# Verbose output
trdc -v git log --oneline -50

# Show command history
trdc --history
trdc --history --full
```

## Streaming Output

TRDC streams the LLM response in real-time, providing immediate feedback as the summary is generated. The final summary is printed to stdout once complete, along with the path to the full output file.

```bash
trdc cargo build
# Output streams incrementally as LLM generates response
# Final summary appears when complete
```

## Exit Code Propagation

TRDC preserves and propagates the exit code from the executed subprocess. This allows scripts and tools to respond to command success or failure appropriately.

```bash
trdc cargo test
echo $?  # Returns exit code from 'cargo test' (0 on success, non-zero on failure)
```

If the LLM summarization fails, TRDC returns exit code 1. If summarization succeeds but the subprocess failed, the subprocess exit code is returned.

## Pre-Filtering

Before sending output to the LLM, TRDC applies pre-filtering to reduce token count and improve summarization quality:

- **ANSI Stripping**: Removes color codes and terminal control sequences
- **Duplicate Collapse**: Reduces 5+ consecutive identical lines to a single marker
- **Whitespace Trimming**: Removes trailing whitespace and leading/trailing blank lines

This filtering happens automatically and is optimized to preserve meaningful content while minimizing token usage.

## Recommended Usage: Pipe Mode

The most reliable way to use TRDC is with **pipe mode** (`command | trdc`). This avoids any argument parsing conflicts and works with any command:

```bash
# Pipe any command output to trdc
cargo test | trdc --context "Focus on failures"
git log --oneline | trdc
sh -c "echo hello" | trdc

# Chain with other tools
grep -r "TODO" . | trdc --context "Summarize TODO items"
curl -s https://api.example.com/data | trdc
```

## Stdin Support

TRDC can read from stdin when no command is provided, allowing you to pipe output from other commands:

```bash
# Pipe log file contents to trdc
cat logs.txt | trdc

# Pipe grep results
grep -r "ERROR" . | trdc --context "Focus on error patterns"

# Pipe curl output
curl -s https://example.com | trdc
```

When using stdin, the command is recorded as `<stdin>` in history.

### Flag Conflicts

TRDC's flags (`--context`, `--gain`, etc.) are now **long-only** to avoid conflicts with command flags:

```bash
# ❌ Before: -c conflicted with sh -c
trdc -c "Focus" sh -c "echo test"

# ✅ After: Works correctly
trdc --context "Focus" sh -c "echo test"

# ✅ Or use pipe mode (recommended)
sh -c "echo test" | trdc --context "Focus"
```

## Token Tracking with `--gain`

Use the `--gain` flag to display token savings statistics:

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

## Command History

TRDC tracks command history, storing each command with its summary for later reference:

```bash
# Show last 10 commands (compact view)
trdc --history

# Show full history with complete summaries
trdc --history --full
```

History is stored in `~/.local/share/trdc/history.jsonl` as JSON Lines format.

## Configuration

Create `~/.config/trdc/config.toml`:

```toml
[llm]
endpoint_url = "http://localhost:1234/v1/chat/completions"
model_name = "local-model"
timeout_secs = 60
max_output_tokens = 512
max_input_chars = 8000
temperature = 0.7
```

### Environment Variables

- `TRDC_LLM_ENDPOINT` - LLM server URL
- `TRDC_LLM_MODEL` - Model name
- `TRDC_MAX_TOKENS` - Max output tokens
- `TRDC_TIMEOUT` - Request timeout in seconds

## Comparison with RTK

| Aspect | RTK | TRDC v2 |
| ------- | ---- | ----- |
| Primary Method | Deterministic filters + Optional LLM | LLM with pre-filtering |
| Context Awareness | Basic command detection | Rich user context |
| Token Savings | 60-90% | 70-95% (with context) |
| Speed | <10ms (Layer 1) | 2-10s (LLM bound) |
| Streaming | No | Yes (real-time feedback) |
| Exit Codes | No | Yes (propagated) |
| Stdin Support | No | Yes |
| History | No | Yes (with `--history`) |
| Use Case | General CLI optimization | Context-aware analysis |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0))
- MIT license ([LICENSE-MIT](http://www.opensource.org/licenses/MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.