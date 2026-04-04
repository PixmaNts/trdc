# TRDC v2 Learnings

## [2026-04-03T17:51:08Z] Session Start
- Plan: trdc-v2 — Async Rewrite + 5 Features
- Total tasks: 7 implementation + 4 final verification
- Critical path: Task 1 → Task 3 → Task 6 → Task 7 → Final Wave

## Execution Waves
- Wave 1: Task 1 (Async Migration) — BLOCKS EVERYTHING
- Wave 2: Tasks 2, 3, 4, 5 (parallel after Wave 1)
- Wave 3: Task 6 (History — depends on Task 3 for exit_code)
- Wave 4: Task 7 (Documentation)
- Final Wave: F1-F4 (parallel verification reviews)

## Key Constraints (Must NOT)
- No retry logic for LLM calls
- No connection pooling or request cancellations
- No WebSocket support (SSE only via HTTP streaming)
- No command-aware filtering rules
- No history search/filter/export features
- No configurable filter thresholds
- No piping stdin TO subprocess — stdin is for trdc's own input
- No tokio/reqwest `full` features — minimal feature sets only
- No changes to `--gain` display format
- No refactoring test helpers (update signatures only)
- No touching chunking algorithm

## Task 1: Async Migration (2026-04-03)

### Key Learnings

1. **spawn_blocking ownership**: When using `tokio::task::spawn_blocking()`, need to clone variables before the closure if they're used later in the async function. Cannot move entire structs if fields are needed later.

2. **reqwest vs ureq API differences**:
   - ureq: `Agent::config_builder().timeout_global(Some(timeout)).build().into()`
   - reqwest: `Client::builder().timeout(timeout).build()`
   - Error handling differs: reqwest has `.is_timeout()`, `.is_status()` methods
   - Response parsing: reqwest has `.json::<T>().await?` instead of manual body reading

3. **Async function signature updates**: When converting functions to async, all call sites must add `.await`. Tests calling sync helper functions don't need #[tokio::test].

4. **Minimal dependency features**: Using minimal feature sets (rt-multi-thread, macros, io-util for tokio; json, stream for reqwest) reduces compile time and binary size.

5. **LSP stale errors**: After replacing ureq with reqwest, LSP may show stale errors from old code until next check.

### Successful Patterns

- Cloning command/args before spawn_blocking to avoid ownership issues
- Keeping Config::load() and SessionStats sync (std::fs) as startup/shutdown ops
- Preserving all existing test functions unchanged (they test sync helpers)
- Using `.context()` for error messages consistently


## Task 4: Smart Pre-filtering (2026-04-03)

### Key Learnings

1. **Regex crate for ANSI stripping**: The `regex` crate handles both CSI (`\x1b[...letter`) and OSC (`\x1b]...BEL` or `\x1b]...\x1b\\`) sequences. Using `unwrap()` on `Regex::new()` is acceptable since patterns are static and compile-time validated.

2. **Line iteration vs indexing**: Clippy warns when loop variables are only used for indexing (`for i in 1..lines.len()`). Prefer `for &line in &lines[1..]` pattern for cleaner iteration.

3. **Empty line handling in trim_whitespace**: When trimming blank lines, need to handle the case where ALL lines are empty. Using `position()` and `rposition()` with `Option` pattern matching is cleaner than `unwrap_or()` + range checks:
   ```rust
   match (start, end) {
       (Some(s), Some(e)) => trimmed[s..=e].join("\n"),
       _ => String::new(),
   }
   ```

4. **Pre-existing bugs in codebase**: Task 1 left compilation errors (stdin reading, is_terminal trait). Had to fix these to test filter module. The `read_to_string` returns `Result<usize>` not `Result<String>` — the buffer is passed by reference.

5. **Filter ordering matters**: strip_ansi → collapse_duplicates → trim_whitespace is the optimal order. Stripping ANSI first prevents false positives in duplicate detection. Trimming whitespace last ensures clean output.

### Implementation Notes

- Filter threshold hardcoded to 5 (not configurable per task constraints)
- Filter applied to `filtered_output` variable after empty check but before chunking
- Both dry-run and summarize paths use filtered output
- 15 filter tests covering all edge cases (ANSI, duplicates, whitespace, integration)

### Verification (2026-04-03)
- Implementation already complete when inspected (src/filter.rs, mod filter in main.rs, regex dep in Cargo.toml)
- All 33 tests pass: `cargo test` → 33 passed (1 suite, 0.00s)
- LSP diagnostics: No errors in filter.rs or main.rs
- Filter pipeline confirmed: pre_filter() called at line 122 before chunk_output()

## Task 2: Piped Stdin Support (2026-04-03)

### Key Learnings

1. **Stdin detection already implemented**: The codebase already had stdin detection using `std::io::IsTerminal` (no external crate needed for edition 2024). The `command` field in `Cli` struct was already `Option<String>`.

2. **Testable validation pattern**: Extracting stdin validation into a pure function `validate_stdin_data() -> Result<Option<String>, &'static str>` makes it testable without `std::process::exit()` which can't be caught by `#[should_panic]`.

3. **Borrow-after-move in streaming**: When assigning a `String` to another variable and then using it again, capture the derived value (e.g., `chars().count()`) before the move.

4. **Missing trait import**: `std::io::Write` must be explicitly imported for `.flush()` to work on `Stdout`/`Stderr`, even though the trait is in std.

5. **3-argument build_output**: The `llm::build_output()` function signature changed to include a `skip_summary` bool parameter (for streaming mode). Call sites must pass 3 arguments.

### Implementation Notes

- 5 new tests added: valid data, empty, whitespace-only, exceeds limit, at-limit
- Total tests: 43 (was 33)
- `echo "hello" | trdc --dry-run` works correctly
- No `--stdin` flag — auto-detection only
- 10MB size limit enforced via `validate_stdin_data()`
- `<stdin>` sentinel used for `user_cmd` when no command provided

## Task 6: History & Persistence (2026-04-03)

### Key Learnings

1. **XDG data dir pattern**: Following tracking.rs exactly for `get_history_path()` using `dirs::data_local_dir()` to get `~/.local/share/trdc/history.jsonl`.

2. **JSONL append pattern**: Using `OpenOptions::new().append(true).create(true).open()` with `BufWriter` for atomic-ish appends. O_APPEND is atomic for small writes on Linux.

3. **chrono for timestamps**: Using `chrono::Local::now().to_rfc3339()` for ISO 8601 timestamps - already a dependency from tracking.rs.

4. **Module declaration order**: New module `mod history;` must be declared before `mod llm;` and `mod tracking;` if using `use history::...` in imports (alphabetical isn't required but logical grouping helps).

5. **Early return pattern for flags**: When `show_history` is true, load and display history at start of `run()`, then return early with `Ok(0)`.

### Implementation Notes

- Created `src/history.rs` with `HistoryEntry` struct (timestamp, command, summary)
- `append()` opens file in append mode, writes JSON line with newline
- `load(limit)` reads lines, skips malformed JSON with `.ok()`, returns last N entries
- `display_history()` shows compact (date | cmd | summary[:50]...) or full (separator + all fields)
- CLI flags: `--history`/`-H` (show compact last 10) and `--full` (show all entries)
- After successful summarization, `history::append()` is called with the entry
- 5 history tests: serialization, compact display, full display, empty display, long summary truncation

### Verification (2026-04-03)
- All 48 tests pass: `cargo test` → 48 passed (1 suite, 0.01s)
- LSP diagnostics: No errors in main.rs, cli.rs, or history.rs
- Build succeeds: `cargo build` → compiled successfully

## Task 8: Documentation & Cleanup (2026-04-03)

### Key Learnings

1. **README sections added**: Streaming output, exit code propagation, pre-filtering, stdin support, command history. Updated comparison table with new v2 features.

2. **AGENTS.md updates**: Added filter.rs and history.rs to STRUCTURE table, added new symbols to CODE MAP, updated WHERE TO LOOK table, updated NOTES with new dependencies (reqwest, tokio, regex, is-terminal).

3. **Clippy warnings fixed**: 
   - history.rs: Changed from `if let Ok(line) = line` to explicit match with `continue` on error
   - main.rs: Combined nested if statements into single `if let Err(e) = ... && cli.verbose`

4. **Verification commands**: All pass - clippy (0 warnings), fmt (formatted), test (48 passed), build --release (compiled).

### Implementation Notes

- README.md: Added v2.0 version reference, 5 new feature sections, updated comparison table
- AGENTS.md: Added filter.rs and history.rs modules with key symbols, updated dependencies in NOTES
- Fixed clippy warnings in history.rs (line iteration pattern) and main.rs (combined if statement)
- cargo fmt required after main.rs edit to fix multi-line if statement formatting
