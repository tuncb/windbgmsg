// winproc.rs
// Windows process utilities for finding process ID by name
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};

use crate::processiter::ProcessIterator;
use crate::winapi::{
    BUF_SIZE, CreateEventW, CreateFileMappingW, DBWIN_BUFFER, DBWIN_BUFFER_READY, DBWIN_DATA_READY,
    DBWinBuffer, FILE_MAP_READ, INFINITE, MapViewOfFile, OpenEventW, OpenFileMappingW,
    PAGE_READWRITE, SetEvent, UnmapViewOfFile, WAIT_OBJECT_0, WaitForSingleObject,
    winapi_get_last_error,
};

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

pub fn capture_debug_output(target_pid: u32) -> Result<(), u32> {
    unsafe {
        // Try to open or create events and file mapping
        let ready_event = open_or_create_event(DBWIN_BUFFER_READY)?;
        let data_event = open_or_create_event(DBWIN_DATA_READY)?;
        let file_mapping = open_or_create_file_mapping(DBWIN_BUFFER)?;

        let buffer_ptr = MapViewOfFile(file_mapping, FILE_MAP_READ, 0, 0, BUF_SIZE);
        if buffer_ptr.is_null() {
            return Err(winapi_get_last_error());
        }

        let dbwin_buffer: *const DBWinBuffer = buffer_ptr as *const DBWinBuffer;
        loop {
            SetEvent(ready_event);
            let wait_result = WaitForSingleObject(data_event, INFINITE);
            if wait_result == WAIT_OBJECT_0 {
                let pid = (*dbwin_buffer).process_id;
                if pid == target_pid {
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
        Ok(())
    }
}
