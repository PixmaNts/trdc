# TRDC v2 Decisions

## [2026-04-03T17:51:08Z] Initial Decisions (from plan)

### Async Migration
- Full async rewrite with `#[tokio::main]` on `main()`
- reqwest (v0.12+) with features: `json`, `stream`
- tokio (v1) with features: `rt-multi-thread`, `macros`, `io-util`
- Add `futures-util` for StreamExt
- `executor::execute()` stays sync, wrapped in `spawn_blocking()`
- `Config::load()` and `SessionStats::save()` stay sync (file I/O at startup/shutdown)

### Streaming
- Always on (no flag)
- Only final combined summary is streamed (multi-chunk intermediate summaries are silent)

### Pre-filtering
- Conservative: ANSI stripping, collapse 5+ identical consecutive lines, trim trailing whitespace
- Collapse marker: `[N identical lines collapsed]`

### Stdin Mode
- Exclusive mode: if stdin has data, skip command execution
- Use `std::io::IsTerminal` (available in Rust 1.70+, edition 2024 requires 1.85+)
- stdin size limit: 10MB default

### History
- JSONL format, persisted to `~/.local/share/trdc/history.jsonl`
- `--history` flag shows last 10 runs compact
- `--history --full` shows complete
- Skip malformed lines on load

### Exit Code Propagation
- `executor::execute()` returns `Result<(String, Option<i32>)>` — already implemented
- Exit code semantics in `main.rs`:
  - Subprocess fail (code N) + LLM success → exit N
  - Subprocess fail (code N) + LLM fail → exit N (prioritize subprocess code)
  - Subprocess success + LLM success → exit 0
  - Subprocess success + LLM fail → exit 1
- `std::process::exit()` used in `main()` after all output and --gain display
- All 33 tests pass
