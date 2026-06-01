# windbgmsg

This is a Rust console application that reads a process name or PID from the user via the command line and captures debug output from matching processes. If a process name is provided, it finds all currently running processes with that executable name before capturing. With `--follow-name`, it keeps refreshing that process set so new and restarted matching processes are captured too. If no process name or PID is given, it captures debug output from all processes. If any Windows API call fails, the application will print the error code and exit.

While capturing, press `Esc` to exit the application.

Captured messages are written with a local timestamp and PID:
```text
[2026-06-01 09:08:07.006] [1234] message
```

## How to run

1. Build the project:
   ```pwsh
   cargo build
   ```
2. Run the project with the process name or PID as an argument (optional):
   ```pwsh
   cargo run -- <process_name> [--wait] [--follow-name] [--highlight <word[,word...]>] [-o <file> [--append]]
   cargo run -- --pid <pid> [--highlight <word[,word...]>] [-o <file> [--append]]
   ```
   Replace `<process_name>` with the name of the executable you want to monitor (e.g., `notepad.exe`). All currently running processes with that executable name will be monitored.
   Use `--pid <pid>` if you already know the target process ID.
   - If you omit the argument, debug output from all processes will be captured:
     ```pwsh
     cargo run --
     ```
   - You can capture a specific PID directly:
     ```pwsh
     cargo run -- --pid 1234
     ```
   - You can add the `--wait` switch to wait for the process to appear if it is not running yet:
     ```pwsh
     cargo run -- notepad.exe --wait
     ```
     The application will wait until at least one matching process starts, then attach to all matching processes found at that time and capture debug output.
   - You can add the `--follow-name` switch to keep tracking matching processes after capture starts:
     ```pwsh
     cargo run -- notepad.exe --follow-name
     ```
     The application will update the captured PID set as matching processes start, exit, or restart.
   - You can write captured debug output to a file with `--output <file>` or `-o <file>`:
     ```pwsh
     cargo run -- notepad.exe --output debug.log
     ```
     By default the file is replaced when capture starts. Add `--append` to append captured debug output to an existing file:
     ```pwsh
     cargo run -- notepad.exe --output debug.log --append
     ```
   - You can highlight matching words in blue on stdout with `--highlight <word[,word...]>`:
     ```pwsh
     cargo run -- notepad.exe --highlight error,warn
     ```
     Matching is case-insensitive. When `--output` is used, the file stays plain text without ANSI color codes.
   - If you use `--wait` without specifying a process name, or with `--pid`, the application will print an error and exit.

## Features
- Finds all current process IDs by executable name (case-insensitive)
- Captures debug output from a specific PID with `--pid <pid>`
- Optionally waits for the process to appear using the `--wait` switch
- Optionally follows process names using the `--follow-name` switch
- Optionally writes captured debug output to a file with `--output <file>` / `-o <file>`
- Optionally appends to the output file with `--append`
- Optionally highlights matching words in blue on stdout with `--highlight`
- Adds a local timestamp and PID to each captured message
- Press `Esc` while capturing to exit
- Captures and prints debug output from the target process set, or from all processes if no name is given
- Returns Windows error codes on failure for easier troubleshooting

## Examples
```pwsh
cargo run -- notepad.exe         # Capture output from all current notepad.exe processes
cargo run -- --pid 1234          # Capture output from PID 1234 only
cargo run -- notepad.exe --wait  # Wait for notepad.exe to start, then capture output
cargo run -- notepad.exe --follow-name  # Keep tracking notepad.exe restarts/new instances
cargo run -- notepad.exe -o debug.log  # Write captured output to debug.log
cargo run -- notepad.exe -o debug.log --append  # Append captured output to debug.log
cargo run -- notepad.exe --highlight error,warn  # Highlight matching words in blue
cargo run --                    # Capture output from all processes
```

If the process is not found or a Windows API call fails, an error message and the error code will be displayed.
