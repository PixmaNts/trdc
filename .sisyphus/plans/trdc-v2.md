# TRDC v2 — Async Rewrite + 5 Features

## TL;DR

> **Quick Summary**: Full async migration (ureq → reqwest+tokio) plus 5 new features: always-on streaming output, exit code propagation, smart pre-filtering, piped stdin support, and JSONL command history.
>
> **Deliverables**:
> - Async foundation with reqwest + tokio
> - Streaming LLM output (always on, final summary streamed)
> - Subprocess exit code propagation
> - Pre-filter module (ANSI strip + line collapse)
> - Piped stdin support (exclusive mode)
> - JSONL history with `--history` flag
> - Updated README and AGENTS.md
>
> **Estimated Effort**: Large
> **Parallel Execution**: YES — 4 waves
> **Critical Path**: Task 1 (async) → Tasks 2-5 (parallel features) → Task 6 (history, depends on 3) → Task 7 (docs)

---

## Context

### Original Request
Brainstorm and implement improvements for TRDC, a Rust CLI that runs commands and sends output to a local LLM (OpenAI-compatible API) for semantic summarization.

### Interview Summary
**Key Discussions**:
- User chose **full async rewrite** over partial async — `#[tokio::main]` on `main()`
- Streaming is **always on** (no flag), but only the final combined summary is streamed (multi-chunk intermediate summaries are silent)
- Pre-filtering is **conservative**: ANSI stripping, collapse 5+ identical consecutive lines, trim trailing whitespace
- Piped stdin is **exclusive mode**: if stdin has data, skip command execution
- History uses **JSONL** with minimal schema, exposed via `--history` flag (not subcommand)
- Test strategy: **tests after implementation**

**Research Findings**:
- Existing codebase has LSP errors with ureq v3 API (send_json, Error::Status)
- OpenAI streaming format uses `delta.content` (not `message.content`) — needs new deserialization structs
- reqwest 0.12+ uses hyper v1 / http-body v1 — streaming API differs from older versions
- `trailing_var_arg = true` prevents subcommands — history must be a flag

### Metis Review
**Identified Gaps** (addressed):
- History CLI approach: use `--history` flag, not subcommand (trailing_var_arg constraint)
- Streaming + multi-chunk: stream final summary only, intermediate chunks processed silently
- Exit code semantics: subprocess exit code propagated; LLM failure → exit 1
- Pre-filter collapse marker: `[N identical lines collapsed]`
- stdin detection: `is-terminal` crate + check if stdin is a terminal
- History malformed lines: skip on load
- Streaming always on: output format changes to incremental, `Summary:` prefix printed once before stream starts

---

## Work Objectives

### Core Objective
Transform TRDC from a synchronous prototype into a production-quality async CLI with streaming output, proper exit codes, smart pre-processing, stdin support, and command history.

### Concrete Deliverables
- All 6 existing source files migrated to async
- New `src/filter.rs` module for pre-filtering
- New `src/history.rs` module for JSONL history
- `Cargo.toml` updated: remove `ureq`, add `reqwest` + `tokio` + `is-terminal`
- `README.md` updated with all new features
- `AGENTS.md` updated with new modules and symbols

### Definition of Done
- [ ] `cargo test` passes (17+ existing tests + new feature tests)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo build --release` succeeds
- [ ] All 5 features work end-to-end

### Must Have
- Streaming output always on for LLM responses
- Subprocess exit code propagated via `std::process::exit()`
- Pre-filtering applied before chunking (ANSI strip, line collapse, whitespace trim)
- Stdin mode: if stdin is piped/redirected, use it as input (skip command execution)
- JSONL history persisted to `~/.local/share/trdc/history.jsonl`
- `--history` flag shows last 10 runs compact; `--history --full` shows complete
- All new config fields have `#[serde(default)]` — existing config.toml works unmodified

### Must NOT Have (Guardrails)
- ❌ No retry logic for LLM calls
- ❌ No connection pooling or request cancellation
- ❌ No WebSocket support (SSE only via HTTP streaming)
- ❌ No command-aware filtering rules
- ❌ No history search/filter/search/export features
- ❌ No configurable filter thresholds
- ❌ No piping stdin TO the subprocess — stdin is for trdc's own input
- ❌ No tokio/reqwest `full` features — minimal feature sets only
- ❌ No changes to the `--gain` display format
- ❌ No refactoring of test helpers (update signatures only)
- ❌ No touching the chunking algorithm

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (inline `#[cfg(test)] mod tests`)
- **Automated tests**: Tests after implementation
- **Framework**: Rust built-in test framework (`cargo test`)
- **Tests do NOT require a running LLM server** — only pure logic is tested

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **CLI integration**: Use Bash — run trdc commands, check exit codes, inspect output
- **Unit tests**: Use Bash (`cargo test`) — verify test pass/fail counts
- **File artifacts**: Use Bash — inspect JSONL, config, output files

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — blocks everything):
└── Task 1: Async Migration Foundation [deep]

Wave 2 (After Wave 1 — 4 parallel features):
├── Task 2: Streaming Output (depends: 1) [deep]
├── Task 3: Exit Code Propagation (depends: 1) [quick]
├── Task 4: Smart Pre-filtering (depends: 1) [unspecified-low]
└── Task 5: Piped Stdin Support (depends: 1) [quick]

Wave 3 (After Tasks 1+3 — history needs exit_code):
└── Task 6: History & Persistence (depends: 1, 3) [unspecified-low]

Wave 4 (After ALL tasks — documentation):
└── Task 7: Documentation & Cleanup (depends: 2, 4, 5, 6) [writing]

Wave FINAL (After ALL implementation — 4 parallel reviews):
├── F1: Plan compliance audit (oracle)
├── F2: Code quality review (unspecified-high)
├── F3: Real manual QA (unspecified-high)
└── F4: Scope fidelity check (deep)
→ Present results → Get explicit user okay

Critical Path: Task 1 → Task 3 → Task 6 → Task 7 → F1-F4 → user okay
Parallel Speedup: ~55% faster than sequential
Max Concurrent: 4 (Wave 2)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 2, 3, 4, 5, 6, 7 | 1 |
| 2 | 1 | 7 | 2 |
| 3 | 1 | 6, 7 | 2 |
| 4 | 1 | 7 | 2 |
| 5 | 1 | 7 | 2 |
| 6 | 1, 3 | 7 | 3 |
| 7 | 2, 4, 5, 6 | F1-F4 | 4 |

### Agent Dispatch Summary

- **Wave 1**: 1 task — T1 → `deep`
- **Wave 2**: 4 tasks — T2 → `deep`, T3 → `quick`, T4 → `unspecified-low`, T5 → `quick`
- **Wave 3**: 1 task — T6 → `unspecified-low`
- **Wave 4**: 1 task — T7 → `writing`
- **FINAL**: 4 tasks — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. **Async Migration Foundation**

- [x] 2. **Streaming LLM Output**

- [x] 3. **Exit Code Propagation**

- [x] 4. **Smart Pre-filtering**

- [x] 5. **Piped Stdin Support**

- [x] 6. **History & Persistence**

- [x] 7. **Documentation & Cleanup**

## Final Verification Wave

- [x] F1. **Plan Compliance Audit** — APPROVE (Must Have 7/7, Must NOT Have 11/11, Tasks 7/7)

- [x] F2. **Code Quality Review** — PASS (Build PASS, Clippy PASS, Tests 48/0, Files 8 clean)

- [x] F3. **Real Manual QA** — CONDITIONAL PASS (Scenarios 20/23, Integration 8/8)

- [x] F4. **Scope Fidelity Check** — COMPLIANT (Tasks 7/7, Unaccounted CLEAN)

  **What to do**:
  - Remove `ureq` from `Cargo.toml`, add `reqwest` (v0.12+, features: `json`, `stream`) + `tokio` (v1, features: `rt-multi-thread`, `macros`, `io-util`) + `futures-util` (for StreamExt)
  - Do NOT enable `full` features on tokio or reqwest — minimal only
  - Convert `main()` to use `#[tokio::main]`, make `run()` async
  - Convert `llm::call_llm()` to async — replace ureq with reqwest streaming POST
  - Convert `llm::summarize()` to async (calls call_llm internally)
  - Wrap `executor::execute()` calls in `tokio::task::spawn_blocking()` since `std::process::Command` is sync
  - Update all call sites in `main.rs` for async signatures (`.await` calls)
  - File I/O for config loading (`Config::load()`) and stats persistence (`SessionStats::save()`) can stay sync — they run at startup/shutdown of a CLI tool
  - Create new deserialization structs for reqwest response: replace ureq-specific parsing with reqwest `.json::<T>()`
  - All 17 existing tests must pass after migration
  - `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt --check` must all pass

  **Must NOT do**:
  - Don't change the chunking algorithm
  - Don't add retry logic or connection pooling
  - Don't convert `Config::load()` or `SessionStats` to tokio::fs — keep std::fs
  - Don't refactor test helpers — update signatures only
  - Don't add any new features yet — this is purely the async migration

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Large-scale architectural change touching every module, requires deep understanding of async Rust patterns
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - `playwright`: No browser involved

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (alone)
  - **Blocks**: Tasks 2, 3, 4, 5, 6, 7
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References** (existing code to follow):
  - `src/main.rs:25-133` — The `run()` function that orchestrates the pipeline. This becomes async. Note the call chain: parse CLI → load config → execute → summarize → track.
  - `src/llm.rs:193-250` — Current `call_llm()` with ureq. This is the core function to rewrite with reqwest. Note the request struct (`ChatRequest`, `Message`) and response parsing (`ChatResponse`, `Choice`).
  - `src/llm.rs:76-136` — `summarize()` function that calls `call_llm()` in a loop. This becomes async. Note the chunk processing loop and token accumulation.
  - `src/executor.rs:6-29` — `execute()` stays sync but will be called via `spawn_blocking()`.
  - `src/config.rs:80-93` — `Config::load()` stays sync (file I/O at startup is fine).
  - `src/tracking.rs:55-73` — `SessionStats::load()/save()` stay sync.

  **API/Type References** (contracts to implement against):
  - `src/llm.rs:16-53` — Current request/response types (`ChatRequest`, `Message`, `ChatResponse`, `Choice`, `ResponseMessage`, `Usage`). These need reqwest-compatible equivalents.
  - `Cargo.toml:10-18` — Current dependencies. `ureq = "3.3.0"` gets replaced.

  **External References**:
  - reqwest 0.12 streaming API: https://docs.rs/reqwest/0.12/reqwest/ — Note: reqwest 0.12+ uses hyper v1
  - tokio::task::spawn_blocking: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html — For wrapping sync `execute()`
  - Rust async best practices: https://tokio.rs/tokio/tutorial

  **WHY Each Reference Matters**:
  - `main.rs:run()` — This is THE orchestration function. Every call becomes `.await`. Understanding its flow is essential.
  - `llm.rs:call_llm()` — This is where ureq → reqwest happens. The request format stays the same (OpenAI-compatible JSON), only the HTTP client changes.
  - `executor.rs:execute()` — Must NOT be made async. Use spawn_blocking to avoid blocking the tokio runtime.
  - Cargo.toml — Dependency swap is step 1. Adding tokio/reqwest without `full` features keeps binary size reasonable.

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Async migration compiles and all tests pass
    Tool: Bash
    Preconditions: Cargo.toml updated with reqwest + tokio, all source files updated
    Steps:
      1. Run `cargo check` — must succeed with 0 errors
      2. Run `cargo test` — must pass all tests (minimum 17)
      3. Run `cargo clippy` — must have 0 warnings
      4. Run `cargo fmt --check` — must be clean
    Expected Result: All 4 commands exit with code 0
    Failure Indicators: Any command exits non-zero, any test failures
    Evidence: .sisyphus/evidence/task-1-async-migration.txt

  Scenario: Release build succeeds
    Tool: Bash
    Preconditions: Async migration complete
    Steps:
      1. Run `cargo build --release`
      2. Check that binary exists at `target/release/trdc`
    Expected Result: Binary builds successfully
    Failure Indicators: Build errors, missing binary
    Evidence: .sisyphus/evidence/task-1-release-build.txt
  ```

  **Commit**: YES
  - Message: `feat: async migration (ureq → reqwest + tokio)`
  - Files: All .rs files, Cargo.toml, Cargo.lock
  - Pre-commit: `cargo test`

- [ ] 2. **Streaming LLM Output**

  **What to do**:
  - Streaming is **always on** (no flag to disable) — every LLM response is streamed token-by-token to stdout
  - Only the **final combined summary** is streamed to the user. Multi-chunk intermediate summaries (when output exceeds `CHUNK_SIZE` and requires multiple LLM calls) are processed silently — only the last chunk's response is streamed live
  - Add `futures-util` as dependency (already included in Task 1 Cargo.toml changes, but confirm it's present)
  - Create new deserialization structs for OpenAI streaming SSE format:
    ```rust
    #[derive(Debug, Deserialize)]
    struct StreamResponse {
        choices: Vec<StreamChoice>,
    }
    #[derive(Debug, Deserialize)]
    struct StreamChoice {
        delta: Delta,
        // finish_reason: Option<String>, // not needed for our use
    }
    #[derive(Debug, Deserialize)]
    struct Delta {
        content: Option<String>,
    }
    ```
  - Key difference from non-streaming: streaming uses `delta.content` (incremental tokens), NOT `message.content` (full message)
  - Modify `call_llm()` to support streaming:
    - Send request with `"stream": true` in the JSON body
    - Use `reqwest::Response::bytes_stream()` to get a `Stream<Item = Result<Bytes, Error>>`
    - Parse SSE lines: look for `data: ` prefix, extract JSON payload
    - Handle `data: [DONE]` as stream terminator
    - Collect full response text while yielding tokens incrementally
  - Create a new `call_llm_streaming()` function (or add a `stream: bool` parameter to `call_llm()`):
    - Signature: `async fn call_llm_streaming(config: &LlmConfig, prompt: &str, mut on_token: impl FnMut(&str)) -> Result<String>`
    - `on_token` callback prints each token to stdout immediately
    - Returns the full accumulated response text (needed for token counting and history)
  - Update `summarize()` in `llm.rs`:
    - For intermediate chunks (not the last): use silent `call_llm()` (no streaming, no output)
    - For the final chunk (or single-chunk case): use `call_llm_streaming()` with a callback that prints tokens
    - Print `Summary:\n` prefix ONCE before streaming begins (not per-chunk)
  - Update `build_output()` in `llm.rs`:
    - When streaming, the summary was already printed — don't print it again
    - Still print the "Full output saved to: ..." footer after streaming completes
  - Streaming output should go to stdout (not stderr) — this is the main user-visible output
  - Handle edge cases:
    - LLM server doesn't support streaming (falls back to non-streaming gracefully)
    - Empty `delta.content` (some providers send empty deltas)
    - Malformed SSE lines (skip them, log to stderr)
  - Add `stream` field to `LlmConfig` with `#[serde(default = "default_true")]` — always true, but configurable for future use
  - All existing tests must still pass (tests don't require an LLM server, so they test non-streaming paths)

  **Must NOT do**:
  - Do NOT add a `--stream` / `--no-stream` CLI flag — streaming is always on
  - Do NOT stream intermediate chunk summaries — only the final combined summary
  - Do NOT add retry logic for failed streams
  - Do NOT add connection pooling or request cancellation
  - Do NOT use WebSocket — HTTP SSE only
  - Do NOT change the chunking algorithm

  **Recommended Agent Profile**:
  - **Category**: `deep`
    - Reason: Complex async streaming with SSE parsing, reqwest byte stream handling, and careful integration with the existing multi-chunk summarize pipeline
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 3, 4, 5)
  - **Blocks**: Task 7 (docs)
  - **Blocked By**: Task 1 (async foundation)

  **References**:

  **Pattern References**:
  - `src/llm.rs:193-250` — Current `call_llm()` with ureq. This is the function to create a streaming version of. Note the request body structure and response parsing.
  - `src/llm.rs:76-136` — `summarize()` function with the chunk processing loop. This needs updating: intermediate chunks silent, final chunk streamed. Note the `chunks.len()` and loop index — use these to detect the final chunk.
  - `src/llm.rs:140-157` — `build_output()` function. When streaming, the summary body was already printed. This function should only output the footer.
  - `src/llm.rs:16-53` — Current request/response types. The streaming types (`StreamResponse`, `StreamChoice`, `Delta`) follow the same serde pattern.

  **API/Type References**:
  - `reqwest::Response::bytes_stream()` — Returns an async stream of byte chunks for SSE parsing
  - `futures_util::StreamExt` — `.next().await` on the byte stream
  - OpenAI Chat Completions Streaming format: `data: {"choices":[{"delta":{"content":"token"}}]}` with `data: [DONE]` terminator
  - `src/llm.rs:16-53` — Existing `ChatRequest`, `Message` structs (reuse for streaming request, add `"stream": true`)

  **External References**:
  - reqwest streaming: https://docs.rs/reqwest/0.12/reqwest/struct.Response.html#method.bytes_stream
  - OpenAI streaming format: https://platform.openai.com/docs/api-reference/chat/create#chat-create-stream
  - futures-util StreamExt: https://docs.rs/futures-util/latest/futures_util/stream/trait.StreamExt.html

  **WHY Each Reference Matters**:
  - `call_llm()` is the core — the streaming version mirrors its request structure but parses SSE incrementally instead of a single JSON response
  - `summarize()` controls when streaming happens — the "only final chunk streams" logic lives here
  - `build_output()` must be aware of streaming to avoid double-printing the summary
  - OpenAI `delta.content` vs `message.content` is the critical deserialization difference — getting this wrong means no tokens

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Streaming types deserialize correctly
    Tool: Bash
    Preconditions: Streaming structs added to llm.rs
    Steps:
      1. Run `cargo test stream`
      2. Verify tests parse `data: {"choices":[{"delta":{"content":"hello"}}]}` correctly
      3. Verify `delta.content` is extracted as "hello"
    Expected Result: Streaming deserialization tests pass
    Failure Indicators: Test failure, wrong field accessed (message vs delta)
    Evidence: .sisyphus/evidence/task-2-stream-types.txt

  Scenario: SSE parsing handles edge cases
    Tool: Bash
    Preconditions: SSE parsing logic implemented
    Steps:
      1. Run `cargo test sse`
      2. Verify: empty lines skipped, "data: [DONE]" terminates, malformed JSON skipped
    Expected Result: All SSE edge case tests pass
    Failure Indicators: Test failure, panic on malformed input
    Evidence: .sisyphus/evidence/task-2-sse-parsing.txt

  Scenario: summarize() only streams final chunk
    Tool: Bash
    Preconditions: summarize() updated with streaming logic
    Steps:
      1. Run `cargo test summarize`
      2. Verify that multi-chunk scenario only calls streaming on the last chunk
    Expected Result: Intermediate chunks processed silently, final chunk streamed
    Failure Indicators: Streaming triggered on intermediate chunks
    Evidence: .sisyphus/evidence/task-2-final-stream.txt

  Scenario: All existing tests still pass
    Tool: Bash
    Preconditions: Streaming changes complete
    Steps:
      1. Run `cargo test`
      2. Verify all 17+ tests pass (no regressions from streaming changes)
    Expected Result: All tests pass
    Failure Indicators: Any test failure
    Evidence: .sisyphus/evidence/task-2-no-regression.txt
  ```

  **Commit**: YES
  - Message: `feat: streaming LLM output (always on)`
  - Files: src/llm.rs, src/main.rs, src/cli.rs, Cargo.toml (if futures-util not in Task 1)
  - Pre-commit: `cargo test`

- [ ] 3. **Exit Code Propagation**

  **What to do**:
  - Change `executor::execute()` return type from `Result<String>` to `Result<(String, Option<i32>)>` — returns (output, exit_code)
  - Exit code is `output.status.code()` from `std::process::Command` — already captured but discarded
  - Update `main.rs` to destructure the tuple: `let (output, exit_code) = executor::execute(...)?;`
  - Define exit code semantics for `trdc`:
    - Subprocess success + LLM success → exit 0
    - Subprocess fail (code N) + LLM success → exit N (propagate subprocess code)
    - Subprocess success + LLM failure → exit 1 (trdc error)
    - Subprocess fail (code N) + LLM failure → exit N (prioritize subprocess code)
  - Store the exit code early, then print output, then call `std::process::exit(code)` at the very END of `run()` (after all output and --gain display)
  - Use `std::process::ExitCode` (Rust 1.61+) for cleaner exit handling if preferred, otherwise `std::process::exit()`
  - Note: `std::process::exit()` bypasses destructors — this is acceptable for a CLI tool
  - Update existing executor tests for new return type
  - Ensure exit code propagation works correctly with streaming (exit happens AFTER stream completes)

  **Must NOT do**:
  - Do NOT change the `--gain` display behavior
  - Do NOT add retry logic on subprocess failure

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small, focused change — two files, clear semantics
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 4, 5)
  - **Blocks**: Task 6 (history needs exit_code)
  - **Blocked By**: Task 1 (async foundation)

  **References**:

  **Pattern References**:
  - `src/executor.rs:6-29` — Current execute() signature and return type. Must change to return tuple.
  - `src/executor.rs:23-26` — Exit code currently printed to stderr but not returned. This is where `output.status.code()` is accessed.
  - `src/main.rs:70` — Call site: `let output = executor::execute(&cli.command, &cli.args)?;` — must destructure tuple.

  **API/Type References**:
  - `std::process::ExitCode` — Rust stable API for exit codes (Rust 1.61+)
  - `std::process::Command::output()` — Returns `Output { status: ExitStatus, stdout, stderr }`

  **WHY Each Reference Matters**:
  - executor.rs is the ONLY file that needs a signature change — all other changes are in main.rs call sites
  - The exit code is already captured (line 24: `output.status.code().unwrap_or(1)`) — just needs to be returned

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Exit code propagated from successful command
    Tool: Bash
    Preconditions: cargo build --release done
    Steps:
      1. Run `cargo run -- echo "hello"; echo "Exit: $?"`
      2. Check that "Exit: 0" appears
    Expected Result: trdc exits with code 0
    Failure Indicators: Exit code is non-zero
    Evidence: .sisyphus/evidence/task-3-exit-success.txt

  Scenario: Exit code propagated from failing command
    Tool: Bash
    Preconditions: cargo build --release done
    Steps:
      1. Run `cargo run -- sh -c "exit 42"; echo "Exit: $?"`
      2. Check that "Exit: 42" appears
    Expected Result: trdc exits with code 42
    Failure Indicators: Exit code is 0 or 1 (not 42)
    Evidence: .sisyphus/evidence/task-3-exit-fail.txt

  Scenario: Executor tests updated and passing
    Tool: Bash
    Preconditions: executor.rs updated with new return type
    Steps:
      1. Run `cargo test executor`
      2. Verify all executor tests pass
    Expected Result: 2+ tests pass (test_execute_echo, test_execute_invalid_command)
    Failure Indicators: Any test failure, compilation error
    Evidence: .sisyphus/evidence/task-3-executor-tests.txt
  ```

  **Commit**: YES
  - Message: `feat: propagate subprocess exit code`
  - Files: src/executor.rs, src/main.rs
  - Pre-commit: `cargo test`

- [ ] 4. **Smart Pre-filtering**

  **What to do**:
  - Create new file `src/filter.rs` with module doc comment `//! Output pre-filtering for TRDC`
  - Declare module in `src/main.rs`: `mod filter;`
  - Implement three filter functions:
    1. `strip_ansi(input: &str) -> String` — Remove ANSI escape sequences using regex pattern `\x1b\[[0-9;]*[a-zA-Z]` and OSC sequences `\x1b\].*?(?:\x07|\x1b\\)`. Either use `regex` crate (already common in Rust ecosystem) or manually iterate. Recommend adding `regex` crate to dependencies.
    2. `collapse_duplicates(input: &str, threshold: usize) -> String` — When `threshold` (default 5) or more identical consecutive lines appear, keep first `threshold-1` lines + add `[N identical lines collapsed]` marker. Line-by-line processing.
    3. `trim_whitespace(input: &str) -> String` — Trim trailing whitespace from each line. Remove leading/trailing blank lines.
  - Public function `pre_filter(input: &str) -> String` that chains all three: `strip_ansi` → `collapse_duplicates` → `trim_whitespace`
  - Apply `pre_filter()` in `main.rs` BEFORE `llm::chunk_output()` in the pipeline — this reduces chunks and tokens
  - Pre-filtering is always on (no flag to disable in this version — add `--no-filter` if guardrails specify, but current spec says no)

  **Must NOT do**:
  - Do NOT add command-aware filtering (e.g., "for cargo test, filter passed tests")
  - Do NOT add configurable filter thresholds (hardcode threshold=5)
  - Do NOT add URL/email/PII redaction
  - Do NOT add filter statistics logging

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: New module with pure functions, straightforward string processing
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 3, 5)
  - **Blocks**: Task 7 (docs)
  - **Blocked By**: Task 1 (async foundation)

  **References**:

  **Pattern References**:
  - `src/llm.rs:160-191` — `chunk_output()` function where pre-filtering should be applied BEFORE this is called
  - `src/main.rs:84,100` — Two places where `output` is used before chunking: the dry-run path and the summarize path. Both need pre-filter applied.

  **API/Type References**:
  - `regex::Regex` — For ANSI stripping pattern. Add `regex` crate to Cargo.toml.
  - Alternative: `strip-ansi-escapes` crate — purpose-built, but adds another dependency. Recommend `regex` since it's more general-purpose.

  **WHY Each Reference Matters**:
  - chunk_output is the boundary — filtering must happen before it to reduce chunk count
  - main.rs has two code paths (dry-run vs real) that both need the filtered output

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: ANSI stripping removes color codes
    Tool: Bash
    Preconditions: filter.rs created with strip_ansi()
    Steps:
      1. Run `cargo test strip_ansi`
      2. Verify test passes with input containing \x1b[31m and \x1b[0m codes
    Expected Result: Test passes, ANSI codes removed from output
    Failure Indicators: Test failure
    Evidence: .sisyphus/evidence/task-4-ansi-strip.txt

  Scenario: Duplicate line collapsing works
    Tool: Bash
    Preconditions: filter.rs created with collapse_duplicates()
    Steps:
      1. Run `cargo test collapse_duplicates`
      2. Verify: 10 identical lines → 4 lines + "[6 identical lines collapsed]"
    Expected Result: Test passes, correct collapse marker and count
    Failure Indicators: Test failure, wrong count in marker
    Evidence: .sisyphus/evidence/task-4-collapse.txt

  Scenario: Pre-filter is applied before chunking
    Tool: Bash
    Preconditions: main.rs updated to call pre_filter()
    Steps:
      1. Run `cargo test pre_filter`
      2. Verify chaining: ANSI input + duplicates → clean output with collapse markers
    Expected Result: Test passes
    Failure Indicators: Test failure
    Evidence: .sisyphus/evidence/task-4-pre-filter.txt
  ```

  **Commit**: YES
  - Message: `feat: smart pre-filtering (ANSI strip, line collapse)`
  - Files: src/filter.rs (new), src/main.rs, Cargo.toml (regex dep)
  - Pre-commit: `cargo test`

- [ ] 5. **Piped Stdin Support**

  **What to do**:
  - Add `is-terminal` crate to Cargo.toml (lightweight, no-std compatible)
  - At the START of `run()` in main.rs, detect stdin mode:
    ```rust
    use std::io::IsTerminal;
    let stdin_data = if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).ok()?;
        Some(buf)
    } else {
        None
    };
    ```
  - If stdin_data is Some and not empty/whitespace-only: use it as `output`, skip `executor::execute()`
  - If stdin_data is Some but empty: print "No output" and exit (same as current empty output path)
  - If stdin_data is None: proceed with normal command execution
  - When in stdin mode, set `user_cmd` to `<stdin>` for history/tracking/display purposes
  - Add stdin size limit: reject stdin > configurable max (default 10MB = 10_485_760 bytes). If exceeded, print error and exit 1.
  - Update Cli struct: `command` becomes optional when stdin is provided. Handle this in arg parsing — if stdin has data, `command` is not required.

  **Must NOT do**:
  - Do NOT pipe stdin to the subprocess — stdin is for trdc's own input
  - Do NOT add `--stdin` flag — auto-detection only
  - Do NOT support interactive stdin (only piped/redirected)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small, focused change — stdin detection + conditional branching
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 2, 3, 4)
  - **Blocks**: Task 7 (docs)
  - **Blocked By**: Task 1 (async foundation)

  **References**:

  **Pattern References**:
  - `src/main.rs:57-61` — Current command construction. Stdin mode bypasses this entirely.
  - `src/main.rs:70` — `executor::execute()` call. Stdin mode skips this.
  - `src/main.rs:76-79` — Empty output handling. Stdin empty uses same path.
  - `src/cli.rs:38-42` — `command` field is required. In stdin mode, this should be optional.

  **API/Type References**:
  - `std::io::IsTerminal` — Stable since Rust 1.70, trait for checking if a stream is a terminal
  - `std::io::Read::read_to_string()` — Read all of stdin into a String
  - Alternative: `is-terminal` crate (standalone, works on older Rust). Since edition 2024 requires Rust 1.85+, `std::io::IsTerminal` is available.

  **WHY Each Reference Matters**:
  - The stdin check must happen FIRST, before any command execution or arg parsing that requires a command
  - `IsTerminal` is in std since Rust 1.70 — no external crate needed for edition 2024
  - The `<stdin>` sentinel for command name keeps history clean

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: Piped stdin is summarized without command execution
    Tool: Bash
    Preconditions: cargo build --release done, LLM server running
    Steps:
      1. Run `echo "The quick brown fox" | cargo run -- --context "summarize this"`
      2. Verify output is a summary of "The quick brown fox"
      3. Verify no command execution error appears
    Expected Result: Summary of stdin content, no "Failed to execute" error
    Failure Indicators: Command execution attempted, "command not found" errors
    Evidence: .sisyphus/evidence/task-5-stdin-pipe.txt
    Note: If LLM server not running, verify the stdin is captured correctly by checking verbose output

  Scenario: Empty stdin produces "No output"
    Tool: Bash
    Preconditions: cargo build done
    Steps:
      1. Run `echo "" | cargo run --verbose`
      2. Verify output contains "No output"
    Expected Result: "No output" response
    Failure Indicators: Error or crash
    Evidence: .sisyphus/evidence/task-5-stdin-empty.txt

  Scenario: No stdin falls through to command execution
    Tool: Bash
    Preconditions: cargo build done
    Steps:
      1. Run `cargo run -- echo "hello"`
      2. Verify command executes normally (no stdin interference)
    Expected Result: Normal command execution + summarization
    Failure Indicators: Stdin mode triggered incorrectly
    Evidence: .sisyphus/evidence/task-5-no-stdin.txt
  ```

  **Commit**: YES
  - Message: `feat: piped stdin support`
  - Files: src/main.rs, src/cli.rs
  - Pre-commit: `cargo test`

- [ ] 6. **History & Persistence**

  **What to do**:
  - Create new file `src/history.rs` with module doc comment `//! Command history tracking for TRDC`
  - Declare module in `src/main.rs`: `mod history;`
  - Define history entry struct:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HistoryEntry {
        pub timestamp: String,  // ISO 8601
        pub command: String,    // "<stdin>" for piped input
        pub summary: String,    // LLM summary text
    }
    ```
  - Implement `append(entry: &HistoryEntry) -> Result<()>`:
    - Open `~/.local/share/trdc/history.jsonl` in append mode (`OpenOptions::new().append(true).create(true)`)
    - Serialize entry as JSON, write as single line + newline
    - Handle concurrent writes gracefully (O_APPEND is atomic for small writes on Linux)
  - Implement `load(limit: Option<usize>) -> Result<Vec<HistoryEntry>>`:
    - Read history.jsonl line by line
    - Skip malformed lines (failed JSON parse) with `serde_json::from_str().ok()`
    - Return last N entries (or all if limit is None)
  - Add CLI flags:
    - `--history` / `-H` flag (boolean): display compact history (last 10) and exit
    - When `--history` AND `--full` are both set: display full history entries
  - In `cli.rs`, add `pub show_history: bool` field with `#[arg(short = 'H', long = "history")]`
  - In `main.rs`, check `cli.show_history` at the start of `run()` — if true, load and display history, then return
  - History display format:
    - Compact: `2026-04-03 14:30 | cargo test | 3 tests passed, 1 failed...`
    - Full: each entry with full summary text, separated by `---`
  - After successful summarization, call `history::append()` with the entry

  **Must NOT do**:
  - Do NOT add history search/filter
  - Do NOT add history rotation/cleanup
  - Do NOT add export formats (CSV, Markdown)
  - Do NOT add configurable history path (use default only)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: New module with straightforward file I/O, CLI flag addition
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (sequential, after Task 3)
  - **Blocks**: Task 7 (docs)
  - **Blocked By**: Task 1 (async), Task 3 (needs exit_code — though schema is minimal without exit_code, the feature benefits from it)

  **References**:

  **Pattern References**:
  - `src/tracking.rs:55-73` — `SessionStats::load()` and `SessionStats::save()` — follow the same file I/O pattern for history
  - `src/tracking.rs:83-86` — `get_stats_path()` — follow same pattern for `get_history_path()`, return `~/.local/share/trdc/history.jsonl`
  - `src/cli.rs:23-24` — `show_gain` flag — follow same pattern for `show_history` flag
  - `src/main.rs:118-124` — Where `show_gain` display happens — add history recording in the same area

  **API/Type References**:
  - `std::fs::OpenOptions::new().append(true).create(true)` — For append-only JSONL writes
  - `serde_json::to_string()` — Serialize entry to single-line JSON
  - `chrono::Local::now().to_rfc3339()` — ISO 8601 timestamp (chrono already in deps)

  **WHY Each Reference Matters**:
  - tracking.rs establishes the pattern for XDG data dir file I/O — follow it exactly
  - The show_gain flag is the template for the show_history flag
  - chrono is already a dependency — no new crate needed for timestamps

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: History entry appended to JSONL file
    Tool: Bash
    Preconditions: history.rs created, main.rs updated
    Steps:
      1. Run `cargo test history_append`
      2. Verify test writes a valid JSONL line to a temp file
      3. Verify the line parses back as valid HistoryEntry JSON
    Expected Result: Test passes, valid JSONL format
    Failure Indicators: Test failure, malformed JSON
    Evidence: .sisyphus/evidence/task-6-history-append.txt

  Scenario: Malformed lines skipped on load
    Tool: Bash
    Preconditions: history.rs load() implemented
    Steps:
      1. Run `cargo test history_load_malformed`
      2. Test creates JSONL with 3 valid + 2 malformed lines
      3. Verify load returns exactly 3 entries
    Expected Result: 3 entries returned, malformed skipped
    Failure Indicators: Test failure, load crashes on malformed data
    Evidence: .sisyphus/evidence/task-6-history-malformed.txt

  Scenario: --history flag shows compact output
    Tool: Bash
    Preconditions: cargo build done, history.jsonl has entries
    Steps:
      1. Run `cargo run -- --history`
      2. Verify output shows last 10 entries in compact format
    Expected Result: Tabular output with timestamp, command, summary preview
    Failure Indicators: Error, no output, wrong format
    Evidence: .sisyphus/evidence/task-6-history-compact.txt

  Scenario: --history --full shows complete output
    Tool: Bash
    Preconditions: cargo build done, history.jsonl has entries
    Steps:
      1. Run `cargo run -- --history --full`
      2. Verify each entry shows full summary text
    Expected Result: Full summaries with separators
    Failure Indicators: Truncated summaries, wrong format
    Evidence: .sisyphus/evidence/task-6-history-full.txt
  ```

  **Commit**: YES
  - Message: `feat: command history (JSONL, --history flag)`
  - Files: src/history.rs (new), src/cli.rs, src/main.rs
  - Pre-commit: `cargo test`

- [ ] 7. **Documentation & Cleanup**

  **What to do**:
  - Update `README.md`:
    - Add streaming output section (always on, final summary streamed)
    - Add exit code propagation section
    - Add pre-filtering section
    - Add stdin support section with examples (`cat logs.txt | trdc`)
    - Add history section with `--history` and `--history --full` examples
    - Update configuration section with any new defaults
    - Update the comparison table (RTK vs TRDC) with new capabilities
    - Update version references
  - Update `AGENTS.md`:
    - Add `src/filter.rs` to STRUCTURE table with key symbols (`pre_filter`, `strip_ansi`, `collapse_duplicates`, `trim_whitespace`)
    - Add `src/history.rs` to STRUCTURE table with key symbols (`HistoryEntry`, `append`, `load`)
    - Add new symbols to CODE MAP
    - Update WHERE TO LOOK table with new modules
    - Update NOTES section with new dependencies (reqwest, tokio, regex, is-terminal)
    - Update COMMANDS section if any changed
  - Run `cargo clippy` and fix any warnings
  - Run `cargo fmt` to ensure formatting
  - Run `cargo test` one final time
  - Verify `cargo build --release` succeeds

  **Must NOT do**:
  - Do NOT add new features during cleanup
  - Do NOT refactor working code for style preferences

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: Documentation-focused task
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (final, after all features)
  - **Blocks**: Final verification wave
  - **Blocked By**: Tasks 2, 4, 5, 6 (all features must be complete)

  **References**:

  **Pattern References**:
  - `README.md` — Current documentation to update
  - `AGENTS.md` — Current knowledge base to update with new modules

  **Acceptance Criteria**:

  **QA Scenarios (MANDATORY):**

  ```
  Scenario: All quality checks pass
    Tool: Bash
    Preconditions: All features implemented
    Steps:
      1. Run `cargo clippy` — 0 warnings
      2. Run `cargo fmt --check` — clean
      3. Run `cargo test` — all pass
      4. Run `cargo build --release` — success
    Expected Result: All 4 commands exit 0
    Failure Indicators: Any non-zero exit
    Evidence: .sisyphus/evidence/task-7-quality-checks.txt

  Scenario: README documents all new features
    Tool: Bash
    Preconditions: README.md updated
    Steps:
      1. Grep README.md for "streaming", "exit code", "pre-filter", "stdin", "history"
      2. Verify each feature has its own section with examples
    Expected Result: All 5 features documented
    Failure Indicators: Missing sections
    Evidence: .sisyphus/evidence/task-7-readme-complete.txt
  ```

  **Commit**: YES
  - Message: `docs: update README and AGENTS.md for v2`
  - Files: README.md, AGENTS.md
  - Pre-commit: `cargo test`

## Final Verification Wave

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
>
> **Do NOT auto-proceed after verification. Wait for user's explicit approval before marking work complete.**

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy` + `cargo fmt --check` + `cargo test`. Review all changed files for: `unwrap()` outside tests, empty catches, unused imports, dead code. Check AI slop: excessive comments, over-abstraction, generic names.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state (`cargo build --release`). Execute EVERY QA scenario from EVERY task. Test cross-feature integration. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built, nothing beyond spec. Check "Must NOT do" compliance. Flag unaccounted changes.
  Output: `Tasks [N/N compliant] | Unaccounted [CLEAN/N files] | VERDICT`

---

## Commit Strategy

| Commit | Message | Key Files | Pre-commit |
|--------|---------|-----------|------------|
| 1 | `feat: async migration (ureq → reqwest + tokio)` | All .rs files, Cargo.toml | `cargo test` |
| 2 | `feat: streaming LLM output (always on)` | src/llm.rs, src/main.rs, src/cli.rs | `cargo test` |
| 3 | `feat: propagate subprocess exit code` | src/executor.rs, src/main.rs | `cargo test` |
| 4 | `feat: smart pre-filtering (ANSI strip, line collapse)` | src/filter.rs (new), src/main.rs | `cargo test` |
| 5 | `feat: piped stdin support` | src/main.rs, src/cli.rs | `cargo test` |
| 6 | `feat: command history (JSONL, --history flag)` | src/history.rs (new), src/cli.rs, src/main.rs | `cargo test` |
| 7 | `docs: update README and AGENTS.md for v2` | README.md, AGENTS.md | `cargo test` |

---

## Success Criteria

### Verification Commands
```bash
cargo test                    # Expected: all tests pass (17 existing + new)
cargo clippy                  # Expected: no warnings
cargo fmt --check             # Expected: clean
cargo build --release         # Expected: success
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] Streaming output works (tokens print incrementally)
- [ ] `trdc sh -c "exit 42"; echo $?` returns 42
- [ ] `echo "hello" | trdc` summarizes stdin without executing a command
- [ ] Pre-filtering strips ANSI and collapses duplicate lines
- [ ] `trdc --history` shows compact history
- [ ] `trdc --history --full` shows full history
