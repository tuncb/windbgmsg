use std::ffi::c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn OpenEventW(dwDesiredAccess: u32, bInheritHandle: i32, lpName: *const u16)
    -> *mut c_void;
    pub fn CreateEventW(
        lpEventAttributes: *mut c_void,
        bManualReset: i32,
        bInitialState: i32,
        lpName: *const u16,
    ) -> *mut c_void;
    pub fn CreateFileMappingW(
        hFile: *mut c_void,
        lpFileMappingAttributes: *mut c_void,
        flProtect: u32,
        dwMaximumSizeHigh: u32,
        dwMaximumSizeLow: u32,
        lpName: *const u16,
    ) -> *mut c_void;
    pub fn OpenFileMappingW(
        dwDesiredAccess: u32,
        bInheritHandle: i32,
        lpName: *const u16,
    ) -> *mut c_void;
    pub fn MapViewOfFile(
        hFileMappingObject: *mut c_void,
        dwDesiredAccess: u32,
        dwFileOffsetHigh: u32,
        dwFileOffsetLow: u32,
        dwNumberOfBytesToMap: usize,
    ) -> *mut c_void;
    pub fn UnmapViewOfFile(lpBaseAddress: *const c_void) -> i32;
    pub fn SetEvent(hEvent: *mut c_void) -> i32;
    pub fn WaitForSingleObject(hHandle: *mut c_void, dwMilliseconds: u32) -> u32;

    pub fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut std::ffi::c_void;
    pub fn Process32FirstW(hSnapshot: *mut std::ffi::c_void, lppe: *mut PROCESSENTRY32W) -> i32;
    pub fn Process32NextW(hSnapshot: *mut std::ffi::c_void, lppe: *mut PROCESSENTRY32W) -> i32;
    pub fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
}

pub const INVALID_HANDLE_VALUE: isize = -1isize;
pub const TH32CS_SNAPPROCESS: u32 = 0x00000002;

#[allow(non_snake_case)]
#[repr(C)]
pub struct PROCESSENTRY32W {
    pub dwSize: u32,
    pub cntUsage: u32,
    pub th32ProcessID: u32,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: u32,
    pub cntThreads: u32,
    pub th32ParentProcessID: u32,
    pub pcPriClassBase: i32,
    pub dwFlags: u32,
    pub szExeFile: [u16; 260],
}
