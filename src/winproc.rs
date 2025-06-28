// winproc.rs
// Windows process utilities for finding process ID by name
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};

use crate::processiter::ProcessIterator;
use crate::winapi::{
    CreateEventW, CreateFileMappingW, MapViewOfFile, OpenEventW, OpenFileMappingW, SetEvent,
    UnmapViewOfFile, WaitForSingleObject,
};

const DBWIN_BUFFER_READY: &str = "DBWIN_BUFFER_READY";
const DBWIN_DATA_READY: &str = "DBWIN_DATA_READY";
const DBWIN_BUFFER: &str = "DBWIN_BUFFER";
const BUF_SIZE: usize = 4096;
const WAIT_OBJECT_0: u32 = 0x00000000;
const INFINITE: u32 = 0xFFFFFFFF;
const FILE_MAP_READ: u32 = 0x0004;
const PAGE_READWRITE: u32 = 0x04;

#[repr(C)]
struct DBWinBuffer {
    pub process_id: u32,
    pub data: [u8; BUF_SIZE - 4],
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

pub fn find_process_id_by_name(app_name: &str) -> Option<u32> {
    ProcessIterator::new()?.find_map(|entry| {
        let exe_name: OsString = OsString::from_wide(&entry.szExeFile);
        let exe_name = exe_name.to_string_lossy();
        let exe_name = exe_name.trim_end_matches(char::from(0));

        if exe_name.eq_ignore_ascii_case(app_name) {
            Some(entry.th32ProcessID)
        } else {
            None
        }
    })
}

pub fn capture_debug_output(target_pid: u32) {
    unsafe {
        // Try to open or create events and file mapping
        let ready_event = OpenEventW(0x1F0003, 0, to_wide(DBWIN_BUFFER_READY).as_ptr());
        let ready_event = if ready_event.is_null() {
            let h = CreateEventW(
                std::ptr::null_mut(),
                0,
                0,
                to_wide(DBWIN_BUFFER_READY).as_ptr(),
            );
            if h.is_null() {
                let err = winapi_get_last_error();
                eprintln!(
                    "[debug] failed to create ready_event, GetLastError: {}",
                    err
                );
            }
            h
        } else {
            ready_event
        };
        let data_event = OpenEventW(0x1F0003, 0, to_wide(DBWIN_DATA_READY).as_ptr());
        let data_event = if data_event.is_null() {
            let h = CreateEventW(
                std::ptr::null_mut(),
                0,
                0,
                to_wide(DBWIN_DATA_READY).as_ptr(),
            );
            if h.is_null() {
                let err = winapi_get_last_error();
                eprintln!("[debug] failed to create data_event, GetLastError: {}", err);
            }
            h
        } else {
            data_event
        };
        let file_mapping = OpenFileMappingW(FILE_MAP_READ, 0, to_wide(DBWIN_BUFFER).as_ptr());
        let file_mapping = if file_mapping.is_null() {
            let h = CreateFileMappingW(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                PAGE_READWRITE,
                0,
                BUF_SIZE as u32,
                to_wide(DBWIN_BUFFER).as_ptr(),
            );
            if h.is_null() {
                let err = winapi_get_last_error();
                eprintln!(
                    "[debug] failed to create file_mapping, GetLastError: {}",
                    err
                );
            }
            h
        } else {
            file_mapping
        };
        if ready_event.is_null() {
            let err = winapi_get_last_error();
            eprintln!("[debug] ready_event is null, GetLastError: {}", err);
        }
        if data_event.is_null() {
            let err = winapi_get_last_error();
            eprintln!("[debug] data_event is null, GetLastError: {}", err);
        }
        if file_mapping.is_null() {
            let err = winapi_get_last_error();
            eprintln!("[debug] file_mapping is null, GetLastError: {}", err);
        }
        if ready_event.is_null() || data_event.is_null() || file_mapping.is_null() {
            eprintln!("Failed to open or create DBWIN objects. Try running as administrator.");
            return;
        }
        let buffer_ptr = MapViewOfFile(file_mapping, FILE_MAP_READ, 0, 0, BUF_SIZE);
        if buffer_ptr.is_null() {
            eprintln!("Failed to map DBWIN_BUFFER.");
            return;
        }
        let dbwin_buffer: *const DBWinBuffer = buffer_ptr as *const DBWinBuffer;
        loop {
            SetEvent(ready_event);
            let wait_result = WaitForSingleObject(data_event, INFINITE);
            if wait_result == WAIT_OBJECT_0 {
                let pid = (*dbwin_buffer).process_id;
                if pid != target_pid {
                    let msg = &(*dbwin_buffer).data;
                    let nul_pos = msg.iter().position(|&c| c == 0).unwrap_or(msg.len());
                    let msg = &msg[..nul_pos];
                    if let Ok(s) = std::str::from_utf8(msg) {
                        println!("[{}] {}", pid, s.trim_end());
                    }
                }
            } else {
                eprintln!("WaitForSingleObject failed or timed out.");
                break;
            }
        }
        UnmapViewOfFile(buffer_ptr);
    }
}

#[inline]
fn winapi_get_last_error() -> u32 {
    unsafe extern "system" {
        fn GetLastError() -> u32;
    }
    unsafe { GetLastError() }
}
