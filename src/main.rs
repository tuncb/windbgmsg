use std::env;
use std::process;

mod processiter;
mod winapi;
mod winproc;
use winproc::find_process_id_by_name;

use crate::winproc::capture_debug_output;

#[derive(Debug)]
struct AppArgs {
    app_name: Option<String>,
    pid: Option<u32>,
    wait: bool,
    help: bool,
}

fn parse_pid(value: &str) -> Result<u32, String> {
    match value.parse::<u32>() {
        Ok(pid) if pid > 0 => Ok(pid),
        Ok(_) => Err("--pid must be greater than 0.".to_string()),
        Err(_) => Err(format!("Invalid PID '{}'.", value)),
    }
}

fn parse_args<I>(args: I) -> Result<AppArgs, String>
where
    I: IntoIterator<Item = String>,
{
    let mut app_name = None;
    let mut pid = None;
    let mut wait = false;
    let mut help = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            help = true;
        } else if arg == "--wait" {
            wait = true;
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

    Ok(AppArgs {
        app_name,
        pid,
        wait,
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
    println!("  {} [process_name] [--wait]", program_name);
    println!("  {} --pid <pid>", program_name);
    println!("  {} --help", program_name);
    println!();
    println!("Arguments:");
    println!("  process_name    Optional executable name to monitor, for example notepad.exe");
    println!();
    println!("Options:");
    println!("  --pid <pid>     Monitor an existing process by PID");
    println!("  --wait          Wait for process_name to start before capturing output");
    println!("  -h, --help      Show this help message and exit");
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

    match (args.app_name, args.pid, args.wait) {
        (Some(app_name), None, true) => {
            // Wait for process to appear
            let pid = loop {
                if let Some(pid) = find_process_id_by_name(&app_name) {
                    break pid;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            };
            println!("Process ID: {}", pid);
            if let Err(e) = capture_debug_output(Some(pid)) {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
        (Some(app_name), None, false) => match find_process_id_by_name(&app_name) {
            Some(pid) => {
                println!("Process ID: {}", pid);
                if let Err(e) = capture_debug_output(Some(pid)) {
                    eprintln!("Error capturing debug output: {}", e);
                    process::exit(1);
                }
            }
            None => {
                eprintln!("Could not find process '{}'.", app_name);
                process::exit(1);
            }
        },
        (None, Some(pid), false) => {
            println!("Process ID: {}", pid);
            if let Err(e) = capture_debug_output(Some(pid)) {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
        (None, None, true) => {
            eprintln!("Error: --wait switch requires an app name.");
            process::exit(1);
        }
        (None, None, false) => {
            println!("No app name provided. Capturing debug output from all processes.");
            if let Err(e) = capture_debug_output(None) {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
        (None, Some(_), true) => unreachable!("validated by get_args"),
        (Some(_), Some(_), _) => unreachable!("validated by get_args"),
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
    }

    #[test]
    fn parses_pid_option() {
        let args = parse(&["--pid", "1234"]).unwrap();
        assert_eq!(args.app_name, None);
        assert_eq!(args.pid, Some(1234));
        assert!(!args.wait);
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
}
