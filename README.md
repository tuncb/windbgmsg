# windbgmsg

This is a Rust console application that reads a process name from the user via the command line, finds its process ID, and captures debug output from that process. If no process name is given, it captures debug output from all processes. If any Windows API call fails, the application will print the error code and exit.

## How to run

1. Build the project:
   ```pwsh
   cargo build
   ```
2. Run the project with the process name as an argument (optional):
   ```pwsh
   cargo run -- <process_name> [--wait]
   ```
   Replace `<process_name>` with the name of the executable you want to monitor (e.g., `notepad.exe`).
   - If you omit the argument, debug output from all processes will be captured:
     ```pwsh
     cargo run --
     ```
   - You can add the `--wait` switch to wait for the process to appear if it is not running yet:
     ```pwsh
     cargo run -- notepad.exe --wait
     ```
     The application will wait until the process starts, then attach and capture debug output.
   - If you use `--wait` without specifying a process name, the application will print an error and exit.

## Features
- Finds the process ID by name (case-insensitive)
- Optionally waits for the process to appear using the `--wait` switch
- Captures and prints debug output from the target process, or from all processes if no name is given
- Returns Windows error codes on failure for easier troubleshooting

## Examples
```pwsh
cargo run -- notepad.exe         # Capture output from notepad.exe only
cargo run -- notepad.exe --wait  # Wait for notepad.exe to start, then capture output
cargo run --                    # Capture output from all processes
```

If the process is not found or a Windows API call fails, an error message and the error code will be displayed.
