use std::env;
use std::process;

mod processiter;
mod winapi;
mod winproc;
use winproc::find_process_id_by_name;

use crate::winproc::capture_debug_output;

struct AppArgs {
    app_name: Option<String>,
    wait: bool,
    help: bool,
}

fn get_args() -> AppArgs {
    let mut app_name = None;
    let mut wait = false;
    let mut help = false;
    let mut args = env::args();
    args.next(); // skip program name
    for arg in args {
        if arg == "--help" || arg == "-h" {
            help = true;
        } else if arg == "--wait" {
            wait = true;
        } else if app_name.is_none() {
            app_name = Some(arg);
        }
    }
    AppArgs {
        app_name,
        wait,
        help,
    }
}

fn print_help(program_name: &str) {
    println!("windbgmsg {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage:");
    println!("  {} [process_name] [--wait]", program_name);
    println!("  {} --help", program_name);
    println!();
    println!("Arguments:");
    println!("  process_name    Optional executable name to monitor, for example notepad.exe");
    println!();
    println!("Options:");
    println!("  --wait          Wait for process_name to start before capturing output");
    println!("  -h, --help      Show this help message and exit");
}

fn main() {
    let program_name = env::args()
        .next()
        .unwrap_or_else(|| "windbgmsg".to_string());
    let args = get_args();
    if args.help {
        print_help(&program_name);
        return;
    }

    match (args.app_name, args.wait) {
        (Some(app_name), true) => {
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
        (Some(app_name), false) => match find_process_id_by_name(&app_name) {
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
        (None, true) => {
            eprintln!("Error: --wait switch requires an app name.");
            process::exit(1);
        }
        (None, false) => {
            println!("No app name provided. Capturing debug output from all processes.");
            if let Err(e) = capture_debug_output(None) {
                eprintln!("Error capturing debug output: {}", e);
                process::exit(1);
            }
        }
    }
}
