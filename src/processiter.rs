use std::mem::{size_of, zeroed};

use crate::winapi::{
    CloseHandle, CreateToolhelp32Snapshot, INVALID_HANDLE_VALUE, PROCESSENTRY32W, Process32FirstW,
    Process32NextW, TH32CS_SNAPPROCESS,
};

pub struct ProcessIterator {
    snapshot: *mut std::ffi::c_void,
    first_call: bool,
}

impl ProcessIterator {
    pub fn new() -> Option<Self> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot as isize == INVALID_HANDLE_VALUE {
                None
            } else {
                Some(ProcessIterator {
                    snapshot,
                    first_call: true,
                })
            }
        }
    }
}

impl Iterator for ProcessIterator {
    type Item = PROCESSENTRY32W;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut entry: PROCESSENTRY32W = zeroed();
            entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

            let success = if self.first_call {
                self.first_call = false;
                Process32FirstW(self.snapshot, &mut entry)
            } else {
                Process32NextW(self.snapshot, &mut entry)
            };

            if success != 0 { Some(entry) } else { None }
        }
    }
}

impl Drop for ProcessIterator {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.snapshot);
        }
    }
}
