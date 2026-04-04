# PROJECT KNOWLEDGE BASE

**Generated:** 2026-04-03
**Branch:** main (no commits yet)

## OVERVIEW

Rust CLI (edition 2024) that runs commands, sends output to a local LLM (OpenAI-compatible API), and prints semantic summaries. Tracks token savings across sessions. Supports streaming output, exit code propagation, pre-filtering, stdin input, and command history.

## STRUCTURE

```
trdc/
├── src/
│   ├── main.rs       # Entry point, pipeline orchestration
│   ├── cli.rs        # Clap derive CLI arg parsing
│   ├── config.rs     # TOML config + env var overrides
│   ├── executor.rs   # Subprocess execution (stdout+stderr capture)
│   ├── llm.rs        # LLM client, chunked processing, streaming, output file persistence
│   ├── tracking.rs   # Token usage tracking, session stats, --gain display
│   ├── filter.rs     # Output pre-filtering (ANSI strip, duplicate collapse, whitespace trim)
│   └── history.rs    # Command history tracking and display
├── Cargo.toml        # Single binary crate, aggressive release profile
└── README.md
```

## WHERE TO LOOK

| Task | File | Key symbols |
|------|------|-------------|
| Add a CLI flag | `src/cli.rs` | `Cli` struct, `#[arg(...)]` |
| Change LLM prompt/behavior | `src/llm.rs` | `build_summarize_prompt()`, `call_llm()` |
| Modify config schema | `src/config.rs` | `Config`, `LlmConfig`, `load()` |
| Change how commands run | `src/executor.rs` | `execute()` |
| Token tracking/stats | `src/tracking.rs` | `SessionStats`, `display_gain()`, `estimate_tokens()` |
| Pipeline flow | `src/main.rs` | `run()` — the main orchestrator |
| Chunking logic | `src/llm.rs` | `chunk_output()`, constants `CHUNK_SIZE=2000`, `MAX_CHUNKS=100` |
| Output pre-filtering | `src/filter.rs` | `pre_filter()`, `strip_ansi()`, `collapse_duplicates()`, `trim_whitespace()` |
| Command history | `src/history.rs` | `HistoryEntry`, `append()`, `load()`, `display_history()` |

## CODE MAP

| Symbol | Type | File | Role |
|--------|------|------|------|
| `Cli` | struct | `cli.rs` | CLI args (clap derive) |
| `Config` | struct | `config.rs` | App config wrapper |
| `LlmConfig` | struct | `config.rs` | LLM endpoint/model/timeout/tokens/temp |
| `execute()` | fn | `executor.rs` | Runs subprocess, returns combined stdout+stderr |
| `summarize()` | fn | `llm.rs` | Chunks output, calls LLM per chunk, accumulates summary |
| `chunk_output()` | fn | `llm.rs` | Splits text at newline boundaries (2KB chunks, 100 max) |
| `call_llm()` | fn | `llm.rs` | HTTP POST to OpenAI-compatible chat completions endpoint |
| `write_output_to_file()` | fn | `llm.rs` | Saves raw output to `/tmp/trdc/trdc_output_{cmd}_{ts}.txt` |
| `SessionStats` | struct | `tracking.rs` | Cumulative token stats, persisted to `~/.local/share/trdc/stats.json` |
| `TokenUsage` | struct | `tracking.rs` | Single-command token counts |
| `display_gain()` | fn | `tracking.rs` | Renders the `--gain` savings table to stderr |
| `estimate_tokens()` | fn | `tracking.rs` | Heuristic: `ceil(chars / 4)` |
| `build_output()` | fn | `llm.rs` | Formats final "Summary:\n...\nFull output saved to: ..." |
| `run()` | fn | `main.rs` | Main pipeline: parse CLI → load config → execute → summarize → track |
| `pre_filter()` | fn | `filter.rs` | Chains all filters: strip_ansi → collapse_duplicates → trim_whitespace |
| `strip_ansi()` | fn | `filter.rs` | Removes ANSI escape sequences (CSI and OSC) |
| `collapse_duplicates()` | fn | `filter.rs` | Collapses 5+ identical consecutive lines |
| `trim_whitespace()` | fn | `filter.rs` | Trims trailing whitespace and leading/trailing blank lines |
| `HistoryEntry` | struct | `history.rs` | Command history entry with timestamp, command, summary |
| `append()` | fn | `history.rs` | Appends entry to history file |
| `load()` | fn | `history.rs` | Loads history entries with optional limit |
| `display_history()` | fn | `history.rs` | Displays history in compact or full mode |

## CONVENTIONS

- **Error handling**: `anyhow::Result` + `.with_context(|| ...)` everywhere. No raw `.unwrap()` outside tests.
- **Logging**: All diagnostic output uses `eprintln!("[trdc] ...")`. `println!` reserved for final output only.
- **Module docs**: Every module has a `//!` doc comment block.
- **Tests**: Inline `#[cfg(test)] mod tests` in each source file. No separate `tests/` directory. Factory helpers named `test_config()` etc.
- **Serde defaults**: All config fields use `#[serde(default = "...")]` with standalone default functions that check env vars.
- **Clap**: `trailing_var_arg = true` + `allow_hyphen_values = true` on root command (needed to pass arbitrary flags through to subprocess).
- **HTTP**: Uses `reqwest` with `tokio` for async streaming. Previously used `ureq` (blocking).
- **Float comparisons**: Tests use epsilon pattern `(actual - expected).abs() < 0.01`.

## ANTI-PATTERNS

- No `rust-toolchain.toml` — edition 2024 requires Rust 1.85+ but this is implicit.
- No CI/CD, no clippy config, no rustfmt config — all defaults.
- Output path hardcoded to `/tmp/trdc/` (not configurable).
- Token estimation is `chars/4` heuristic — not a real tokenizer.

## COMMANDS

```bash
cargo build                  # Debug build
cargo build --release        # Release (LTO, stripped, single codegen unit)
cargo test                   # Run inline unit tests
cargo run -- <cmd> [args...] # Run trdc
cargo clippy                 # Lint (no custom config)
cargo fmt --check            # Format check (no custom config)
```

## NOTES

- Requires a running LLM server (default: LM Studio at `localhost:1234`). Tests do NOT require the server — only pure logic is tested.
- Config: `~/.config/trdc/config.toml` · Stats: `~/.local/share/trdc/stats.json` · History: `~/.local/share/trdc/history.jsonl` · Temp output: `/tmp/trdc/`.
- Env overrides: `TRDC_LLM_ENDPOINT`, `TRDC_LLM_MODEL`, `TRDC_MAX_TOKENS`, `TRDC_TIMEOUT`.
- Release binary is aggressively optimized: `lto=true`, `codegen-units=1`, `panic="abort"`, `strip=true`.
- New dependencies: `reqwest` (async HTTP), `tokio` (async runtime), `regex` (ANSI stripping), `is-terminal` (stdin detection).
- Stdin support: When no command is provided, reads from stdin (max 10MB). Command recorded as `<stdin>` in history.
- Streaming: LLM responses are streamed in real-time for immediate feedback.
- Exit codes: Subprocess exit codes are propagated through to the caller.