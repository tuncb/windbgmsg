# windbgmsg Improvement Ideas

This document captures practical ideas for improving `windbgmsg`, a Rust Windows CLI that listens to `DBWIN_*` debug output, optionally filters by process, and prints matching `OutputDebugString` messages.

## Current Baseline

- The project builds successfully with `cargo check`.
- The code is formatted with `cargo fmt --check`.
- Clippy passes with `cargo clippy --all-targets -- -D warnings`.
- `cargo test` passes, but there are currently no tests.
- The app currently supports:
  - Capturing debug output from all processes.
  - Capturing debug output from one process name.
  - Waiting for a named process with `--wait`.
  - Basic `--help` output.

## High-Value Feature Ideas

### Better Targeting

- Add `--pid <pid>` for direct PID capture.
- Allow multiple process filters, such as repeated `--process name.exe` values.
- Capture all matching process names instead of only the first match.
- Add `--follow-name` to keep tracking new PIDs when a process restarts.
- Add `--list` to show matching processes before capture starts.

### Richer Output

- Add timestamps to each message.
- Support local time, UTC, or epoch timestamp formats.
- Add `--output file.log` for writing captured messages to a file.
- Add `--append` to preserve existing logs.
- Add `--jsonl` for structured line-delimited JSON output.
- Add `--csv` for spreadsheet-friendly output.
- Include process name alongside PID.
- Add colorized console output.
- Add `--raw` for scripting-friendly output.

### Message Filtering

- Add `--contains <text>` to include only matching messages.
- Add `--exclude <text>` to suppress noisy messages.
- Add `--regex <pattern>` for advanced filtering.
- Add `--case-sensitive` for stricter matching.
- Add `--count <n>` to exit after capturing a fixed number of messages.
- Add `--duration <time>` to capture for a bounded time, such as `30s` or `5m`.

### CLI Robustness

- Replace manual argument parsing with `clap` or `lexopt`.
- Reject unknown arguments instead of silently ignoring them.
- Add `--wait-timeout <time>` so `--wait` cannot run forever by accident.
- Print periodic status while waiting for a process to appear.
- Add clearer examples to `--help`.

### Windows API Hardening

- Add RAII handle wrappers so opened events and file mappings are always closed.
- Check failures from `SetEvent`.
- Check failures from `UnmapViewOfFile`.
- Format Win32 errors into readable messages instead of only numeric codes.
- Consider using `windows-sys` instead of hand-maintained FFI definitions.

### Message Decoding

- Current decoding only prints valid UTF-8 messages.
- Add a lossy fallback so invalid UTF-8 does not silently disappear.
- Consider Windows ANSI code page decoding for compatibility with older applications.

### Tests And CI

- Unit-test argument parsing.
- Unit-test message filtering.
- Unit-test output formatting.
- Split formatting/filtering logic away from the Win32 capture loop to make it testable.
- Add a Windows integration test helper that calls `OutputDebugString`.
- Update CI to run:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - `cargo build --release`

## Suggested First Roadmap

1. Add a proper CLI parser with `clap`.
2. Add `--pid`.
3. Add timestamps.
4. Add `--output` and `--jsonl`.
5. Add RAII cleanup for Windows handles.
6. Add parser, filter, and formatter tests.

This roadmap keeps the app small while making it more useful as a diagnostics tool.
