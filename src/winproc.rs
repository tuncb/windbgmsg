// winproc.rs
// Windows process utilities for finding process ID by name
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Write};
use std::mem::zeroed;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::sync::{Arc, RwLock};

use crate::processiter::ProcessIterator;
use crate::winapi::{
    BUF_SIZE, CreateEventW, CreateFileMappingW, DBWIN_BUFFER, DBWIN_BUFFER_READY, DBWIN_DATA_READY,
    DBWinBuffer, FILE_MAP_READ, GetAsyncKeyState, GetLocalTime, MapViewOfFile, OpenEventW,
    OpenFileMappingW, PAGE_READWRITE, SYSTEMTIME, SetEvent, UnmapViewOfFile, VK_ESCAPE,
    WAIT_OBJECT_0, WAIT_TIMEOUT, WaitForSingleObject, winapi_get_last_error,
};

const CAPTURE_WAIT_TIMEOUT_MS: u32 = 100;
const ANSI_BLUE: &str = "\x1b[34m";
const ANSI_DEFAULT_FOREGROUND: &str = "\x1b[39m";

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

pub fn find_process_ids_by_name(app_name: &str) -> Vec<u32> {
    ProcessIterator::new()
        .map(|processes| {
            processes
                .filter_map(|entry| {
                    let exe_name: OsString = OsString::from_wide(&entry.szExeFile);
                    let exe_name = exe_name.to_string_lossy();
                    let exe_name = exe_name.trim_end_matches(char::from(0));

                    if exe_name.eq_ignore_ascii_case(app_name) {
                        Some(entry.th32ProcessID)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn matches_target_pid(target_pids: Option<&HashSet<u32>>, pid: u32) -> bool {
    match target_pids {
        Some(pids) => pids.contains(&pid),
        None => true,
    }
}

pub type SharedTargetPids = Arc<RwLock<HashSet<u32>>>;

pub enum CaptureTarget {
    All,
    StaticPids(HashSet<u32>),
    SharedPids(SharedTargetPids),
}

#[derive(Debug)]
pub enum CaptureError {
    Windows(u32),
    Io(io::Error),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::Windows(code) => write!(f, "Windows error {}", code),
            CaptureError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl From<u32> for CaptureError {
    fn from(value: u32) -> Self {
        CaptureError::Windows(value)
    }
}

impl From<io::Error> for CaptureError {
    fn from(value: io::Error) -> Self {
        CaptureError::Io(value)
    }
}

impl CaptureTarget {
    fn matches_pid(&self, pid: u32) -> bool {
        match self {
            CaptureTarget::All => true,
            CaptureTarget::StaticPids(pids) => matches_target_pid(Some(pids), pid),
            CaptureTarget::SharedPids(pids) => pids
                .read()
                .map(|pids| matches_target_pid(Some(&pids), pid))
                .unwrap_or(false),
        }
    }
}

fn escape_is_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_ESCAPE) < 0 }
}

fn format_timestamp(time: &SYSTEMTIME) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        time.wYear,
        time.wMonth,
        time.wDay,
        time.wHour,
        time.wMinute,
        time.wSecond,
        time.wMilliseconds
    )
}

fn current_timestamp() -> String {
    unsafe {
        let mut time: SYSTEMTIME = zeroed();
        GetLocalTime(&mut time);
        format_timestamp(&time)
    }
}

fn highlight_text(text: &str, words: &[String]) -> String {
    if words.is_empty() {
        return text.to_string();
    }

    let mut highlighted = String::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        let best_match = words
            .iter()
            .filter(|word| !word.is_empty())
            .filter_map(|word| {
                let end = index.checked_add(word.len())?;
                if end <= text.len()
                    && text.is_char_boundary(end)
                    && text[index..end].eq_ignore_ascii_case(word)
                {
                    Some(end)
                } else {
                    None
                }
            })
            .max_by_key(|end| end - index);

        if let Some(end) = best_match {
            highlighted.push_str(ANSI_BLUE);
            highlighted.push_str(&text[index..end]);
            highlighted.push_str(ANSI_DEFAULT_FOREGROUND);
            index = end;
        } else {
            let ch = text[index..].chars().next().expect("index is in bounds");
            highlighted.push(ch);
            index += ch.len_utf8();
        }
    }

    highlighted
}

fn open_or_create_event(name: &str) -> Result<*mut std::ffi::c_void, u32> {
    unsafe {
        let event = OpenEventW(0x1F0003, 0, to_wide(name).as_ptr());
        if event.is_null() {
            let h = CreateEventW(std::ptr::null_mut(), 0, 0, to_wide(name).as_ptr());
            if h.is_null() {
                let err = winapi_get_last_error();
                return Err(err);
            }
            Ok(h)
        } else {
            Ok(event)
        }
    }
}

fn open_or_create_file_mapping(name: &str) -> Result<*mut std::ffi::c_void, u32> {
    unsafe {
        let file_mapping = OpenFileMappingW(FILE_MAP_READ, 0, to_wide(name).as_ptr());
        if file_mapping.is_null() {
            let h = CreateFileMappingW(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                PAGE_READWRITE,
                0,
                BUF_SIZE as u32,
                to_wide(name).as_ptr(),
            );
            if h.is_null() {
                return Err(winapi_get_last_error());
            }
            Ok(h)
        } else {
            Ok(file_mapping)
        }
    }
}

pub fn capture_debug_output(
    target: CaptureTarget,
    output: &mut dyn Write,
    highlight_words: &[String],
) -> Result<(), CaptureError> {
    unsafe {
        // Try to open or create events and file mapping
        let ready_event = open_or_create_event(DBWIN_BUFFER_READY)?;
        let data_event = open_or_create_event(DBWIN_DATA_READY)?;
        let file_mapping = open_or_create_file_mapping(DBWIN_BUFFER)?;

        let buffer_ptr = MapViewOfFile(file_mapping, FILE_MAP_READ, 0, 0, BUF_SIZE);
        if buffer_ptr.is_null() {
            return Err(winapi_get_last_error().into());
        }

        let dbwin_buffer: *const DBWinBuffer = buffer_ptr as *const DBWinBuffer;
        loop {
            if escape_is_pressed() {
                eprintln!("Escape pressed. Exiting.");
                break;
            }

            SetEvent(ready_event);
            let wait_result = WaitForSingleObject(data_event, CAPTURE_WAIT_TIMEOUT_MS);
            if wait_result == WAIT_OBJECT_0 {
                let pid = (*dbwin_buffer).process_id;
                if target.matches_pid(pid) {
                    let msg = &(*dbwin_buffer).data;
                    let nul_pos = msg.iter().position(|&c| c == 0).unwrap_or(msg.len());
                    let msg = &msg[..nul_pos];
                    if let Ok(s) = std::str::from_utf8(msg) {
                        let line = format!("[{}] [{}] {}", current_timestamp(), pid, s.trim_end());
                        writeln!(output, "{}", highlight_text(&line, highlight_words))?;
                        output.flush()?;
                    }
                }
            } else if wait_result == WAIT_TIMEOUT {
                continue;
            } else {
                eprintln!("WaitForSingleObject failed.");
                break;
            }
        }
        UnmapViewOfFile(buffer_ptr);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{SYSTEMTIME, format_timestamp, highlight_text};

    #[test]
    fn formats_timestamp_with_milliseconds() {
        let time = SYSTEMTIME {
            wYear: 2026,
            wMonth: 6,
            wDayOfWeek: 1,
            wDay: 1,
            wHour: 9,
            wMinute: 8,
            wSecond: 7,
            wMilliseconds: 6,
        };

        assert_eq!(format_timestamp(&time), "2026-06-01 09:08:07.006");
    }

    #[test]
    fn highlights_matching_words_case_insensitively() {
        let words = vec!["error".to_string()];

        assert_eq!(
            highlight_text("An ERROR occurred", &words),
            "An \x1b[34mERROR\x1b[39m occurred"
        );
    }

    #[test]
    fn highlights_longest_matching_word_first() {
        let words = vec!["error".to_string(), "error code".to_string()];

        assert_eq!(
            highlight_text("error code 5", &words),
            "\x1b[34merror code\x1b[39m 5"
        );
    }

    #[test]
    fn leaves_text_unchanged_without_highlight_words() {
        assert_eq!(highlight_text("plain output", &[]), "plain output");
    }
}
