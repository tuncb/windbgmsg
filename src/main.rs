use std::collections::HashSet;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

mod processiter;
mod winapi;
mod winproc;
use winproc::find_process_ids_by_name;

use crate::winproc::{CaptureTarget, SharedTargetPids, capture_debug_output};

const PID_SCAN_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
struct AppArgs {
    app_name: Option<String>,
    pid: Option<u32>,
    wait: bool,
    follow_name: bool,
    output_file: Option<PathBuf>,
    append: bool,
    help: bool,
}

fn parse_pid(value: &str) -> Result<u32, String> {
    match value.parse::<u32>() {
        Ok(pid) if pid > 0 => Ok(pid),
        Ok(_) => Err("--pid must be greater than 0.".to_string()),
        Err(_) => Err(format!("Invalid PID '{}'.", value)),
    }
}

fn parse_output_file(value: &str) -> Result<PathBuf, String> {
    if value.is_empty() {
        return Err("--output requires a non-empty file path.".to_string());
    }

    Ok(PathBuf::from(value))
}

fn set_output_file(output_file: &mut Option<PathBuf>, value: &str) -> Result<(), String> {
    if output_file.is_some() {
        return Err("--output can only be specified once.".to_string());
    }

    *output_file = Some(parse_output_file(value)?);
    Ok(())
}

fn parse_args<I>(args: I) -> Result<AppArgs, String>
where
    I: IntoIterator<Item = String>,
{
    let mut app_name = None;
    let mut pid = None;
    let mut wait = false;
    let mut follow_name = false;
    let mut output_file = None;
    let mut append = false;
    let mut help = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            help = true;
        } else if arg == "--wait" {
            wait = true;
        } else if arg == "--follow-name" {
            follow_name = true;
        } else if arg == "--append" {
            append = true;
        } else if arg == "--output" || arg == "-o" {
            let value = args
                .next()
                .ok_or_else(|| "--output requires a file path.".to_string())?;
            set_output_file(&mut output_file, &value)?;
        } else if let Some(value) = arg.strip_prefix("--output=") {
            set_output_file(&mut output_file, value)?;
        } else if let Some(value) = arg.strip_prefix("-o=") {
            set_output_file(&mut output_file, value)?;
        } else if arg == "--pid" {
            let value = args
                .next()
                .ok_or_else(|| "--pid requires a PID value.".to_string())?;
            if pid.is_some() {
                return Err("--pid can only be specified once.".to_string());
            }
            pid = Some(parse_pid(&value)?);
        } else if let Some(value) = arg.strip_prefix("--pid=") {
            if pid.is_some() {
                return Err("--pid can only be specified once.".to_string());
            }
            pid = Some(parse_pid(value)?);
        } else if app_name.is_none() {
            app_name = Some(arg);
        } else {
            return Err(format!("Unexpected argument '{}'.", arg));
        }
    }

    if app_name.is_some() && pid.is_some() {
        return Err("Specify either a process name or --pid, not both.".to_string());
    }

    if wait && pid.is_some() {
        return Err("--wait can only be used with a process name.".to_string());
    }

    if follow_name && pid.is_some() {
        return Err("--follow-name can only be used with a process name.".to_string());
    }

    if follow_name && app_name.is_none() {
        return Err("--follow-name requires a process name.".to_string());
    }

    if append && output_file.is_none() {
        return Err("--append requires --output <file>.".to_string());
    }

    Ok(AppArgs {
        app_name,
        pid,
        wait,
        follow_name,
        output_file,
        append,
        help,
    })
}

fn get_args() -> Result<AppArgs, String> {
    parse_args(env::args().skip(1))
}

fn print_help(program_name: &str) {
    println!("windbgmsg {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage:");
    println!("  {} [process_name] [--wait] [--follow-name]", program_name);
    println!("  {} --pid <pid>", program_name);
    println!("  {} --help", program_name);
    println!();
    println!("Arguments:");
    println!(
        "  process_name    Optional executable name to monitor across all current matching PIDs"
    );
    println!();
    println!("Options:");
    println!("  --pid <pid>     Monitor an existing process by PID");
    println!("  --wait          Wait for process_name to start before capturing output");
    println!("  --follow-name   Keep tracking new and restarted processes matching process_name");
    println!("  -o, --output <file>");
    println!("                  Write captured debug output to a file instead of stdout");
    println!("  --append        Append to --output instead of replacing it");
    println!("  -h, --help      Show this help message and exit");
}

fn format_pids(pids: &HashSet<u32>) -> String {
    if pids.is_empty() {
        return "none".to_string();
    }

    let mut pids: Vec<u32> = pids.iter().copied().collect();
    pids.sort_unstable();
    pids.iter()
        .map(u32::to_string)
        .collect::<Vec<String>>()
        .join(", ")
}

fn current_target_pids(app_name: &str) -> HashSet<u32> {
    find_process_ids_by_name(app_name).into_iter().collect()
}

fn wait_for_target_pids(app_name: &str) -> HashSet<u32> {
    loop {
        let pids = current_target_pids(app_name);
        if !pids.is_empty() {
            return pids;
        }
        thread::sleep(PID_SCAN_INTERVAL);
    }
}

fn start_pid_scanner(app_name: String, target_pids: SharedTargetPids) {
    thread::spawn(move || {
        loop {
            thread::sleep(PID_SCAN_INTERVAL);

            let next_pids = current_target_pids(&app_name);
            let Ok(mut pids) = target_pids.write() else {
                break;
            };

            if *pids != next_pids {
                *pids = next_pids;
                eprintln!(
                    "Updated process IDs for '{}': {}",
                    app_name,
                    format_pids(&pids)
                );
            }
        }
    });
}

fn open_output(output_file: Option<&Path>, append: bool) -> io::Result<Box<dyn Write>> {
    match output_file {
        Some(path) => {
            let mut options = OpenOptions::new();
            options.create(true).write(true);
            if append {
                options.append(true);
            } else {
                options.truncate(true);
            }

            let file: File = options.open(path)?;
            Ok(Box::new(file))
        }
        None => Ok(Box::new(io::stdout())),
    }
}

fn main() {
    let program_name = env::args()
        .next()
        .unwrap_or_else(|| "windbgmsg".to_string());
    let args = match get_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Run '{} --help' for usage.", program_name);
            process::exit(1);
        }
    };
    if args.help {
        print_help(&program_name);
        return;
    }

    let mut output = match open_output(args.output_file.as_deref(), args.append) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Error opening output file: {}", e);
            process::exit(1);
        }
    };

    match (args.app_name, args.pid, args.wait, args.follow_name) {
        (Some(app_name), None, true, follow_name) => {
            let target_pids = wait_for_target_pids(&app_name);
            println!("Process IDs: {}", format_pids(&target_pids));
            let target = if follow_name {
                let shared_pids = Arc::new(RwLock::new(target_pids));
                start_pid_scanner(app_name, Arc::clone(&shared_pids));
                CaptureTarget::SharedPids(shared_pids)
            } else {
                CaptureTarget::StaticPids(target_pids)
            };

            if let Err(e) = capture_debug_output(target, &mut output) {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
        (Some(app_name), None, false, follow_name) => {
            let target_pids = current_target_pids(&app_name);
            if target_pids.is_empty() {
                eprintln!("Could not find process '{}'.", app_name);
                process::exit(1);
            }

            println!("Process IDs: {}", format_pids(&target_pids));
            let target = if follow_name {
                let shared_pids = Arc::new(RwLock::new(target_pids));
                start_pid_scanner(app_name, Arc::clone(&shared_pids));
                CaptureTarget::SharedPids(shared_pids)
            } else {
                CaptureTarget::StaticPids(target_pids)
            };

            if let Err(e) = capture_debug_output(target, &mut output) {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
        (None, Some(pid), false, false) => {
            println!("Process ID: {}", pid);
            let mut target_pids = HashSet::new();
            target_pids.insert(pid);
            if let Err(e) =
                capture_debug_output(CaptureTarget::StaticPids(target_pids), &mut output)
            {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
        (None, None, true, false) => {
            eprintln!("Error: --wait switch requires an app name.");
            process::exit(1);
        }
        (None, None, false, false) => {
            println!("No app name provided. Capturing debug output from all processes.");
            if let Err(e) = capture_debug_output(CaptureTarget::All, &mut output) {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
        (None, Some(_), _, true) => unreachable!("validated by get_args"),
        (None, Some(_), true, false) => unreachable!("validated by get_args"),
        (Some(_), Some(_), _, _) => unreachable!("validated by get_args"),
        (None, None, _, true) => unreachable!("validated by get_args"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    fn parse(args: &[&str]) -> Result<super::AppArgs, String> {
        parse_args(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn parses_process_name() {
        let args = parse(&["notepad.exe"]).unwrap();
        assert_eq!(args.app_name.as_deref(), Some("notepad.exe"));
        assert_eq!(args.pid, None);
        assert!(!args.wait);
        assert!(!args.follow_name);
    }

    #[test]
    fn parses_pid_option() {
        let args = parse(&["--pid", "1234"]).unwrap();
        assert_eq!(args.app_name, None);
        assert_eq!(args.pid, Some(1234));
        assert!(!args.wait);
        assert!(!args.follow_name);
    }

    #[test]
    fn parses_pid_equals_option() {
        let args = parse(&["--pid=1234"]).unwrap();
        assert_eq!(args.pid, Some(1234));
    }

    #[test]
    fn rejects_pid_with_process_name() {
        let err = parse(&["notepad.exe", "--pid", "1234"]).unwrap_err();
        assert!(err.contains("either a process name or --pid"));
    }

    #[test]
    fn rejects_wait_with_pid() {
        let err = parse(&["--pid", "1234", "--wait"]).unwrap_err();
        assert!(err.contains("--wait can only be used with a process name"));
    }

    #[test]
    fn rejects_invalid_pid() {
        let err = parse(&["--pid", "abc"]).unwrap_err();
        assert!(err.contains("Invalid PID"));
    }

    #[test]
    fn parses_follow_name() {
        let args = parse(&["notepad.exe", "--follow-name"]).unwrap();
        assert_eq!(args.app_name.as_deref(), Some("notepad.exe"));
        assert!(args.follow_name);
    }

    #[test]
    fn parses_output_option() {
        let args = parse(&["notepad.exe", "--output", "debug.log"]).unwrap();
        assert_eq!(
            args.output_file.as_deref(),
            Some(std::path::Path::new("debug.log"))
        );
        assert!(!args.append);
    }

    #[test]
    fn parses_output_equals_option() {
        let args = parse(&["--output=debug.log"]).unwrap();
        assert_eq!(
            args.output_file.as_deref(),
            Some(std::path::Path::new("debug.log"))
        );
    }

    #[test]
    fn parses_short_output_option() {
        let args = parse(&["-o", "debug.log"]).unwrap();
        assert_eq!(
            args.output_file.as_deref(),
            Some(std::path::Path::new("debug.log"))
        );
    }

    #[test]
    fn parses_append_with_output() {
        let args = parse(&["notepad.exe", "--output", "debug.log", "--append"]).unwrap();
        assert_eq!(
            args.output_file.as_deref(),
            Some(std::path::Path::new("debug.log"))
        );
        assert!(args.append);
    }

    #[test]
    fn rejects_follow_name_with_pid() {
        let err = parse(&["--pid", "1234", "--follow-name"]).unwrap_err();
        assert!(err.contains("--follow-name can only be used with a process name"));
    }

    #[test]
    fn rejects_follow_name_without_process_name() {
        let err = parse(&["--follow-name"]).unwrap_err();
        assert!(err.contains("--follow-name requires a process name"));
    }

    #[test]
    fn rejects_append_without_output() {
        let err = parse(&["notepad.exe", "--append"]).unwrap_err();
        assert!(err.contains("--append requires --output"));
    }

    #[test]
    fn rejects_duplicate_output() {
        let err = parse(&["--output", "a.log", "--output", "b.log"]).unwrap_err();
        assert!(err.contains("--output can only be specified once"));
    }

    #[test]
    fn rejects_empty_output() {
        let err = parse(&["--output="]).unwrap_err();
        assert!(err.contains("--output requires a non-empty file path"));
    }
}
