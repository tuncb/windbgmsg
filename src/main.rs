use std::env;
use std::process;

mod winproc;
use winproc::find_process_id_by_name;

fn get_app_name_from_args() -> Option<String> {
    let mut args = env::args();
    args.next(); // skip program name
    args.next()
}

fn main() {
    match get_app_name_from_args() {
        Some(app_name) => match find_process_id_by_name(&app_name) {
            Some(pid) => println!("Process ID: {}", pid),
            None => {
                eprintln!("Could not find process '{}'.", app_name);
                process::exit(1);
            }
        },
        None => {
            eprintln!("No app name provided. Please provide an app name as the first argument.");
            process::exit(1);
        }
    }
}
