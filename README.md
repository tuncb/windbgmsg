# windbgmsg

This is a Rust console application that reads a process name from the user via the command line, finds its process ID, and captures debug output from the process. If any Windows API call fails, the application will print the error code and exit.

## How to run

1. Build the project:
   ```pwsh
   cargo build
   ```
2. Run the project with the process name as an argument:
   ```pwsh
   cargo run -- <process_name>
   ```
   Replace `<process_name>` with the name of the executable you want to monitor (e.g., `notepad.exe`).

## Features
- Finds the process ID by name (case-insensitive)
- Captures and prints debug output from the target process
- Returns Windows error codes on failure for easier troubleshooting

## Example
```pwsh
cargo run -- notepad.exe
```

If the process is not found or a Windows API call fails, an error message and the error code will be displayed.
