use std::io::Write;

use {
    bytemuck::Pod,
    once_cell::sync::Lazy,
    std::{env, fs::File, io::Error, sync::OnceLock, ffi::OsString, os::windows::ffi::OsStringExt, mem, path::PathBuf, process::Command},
    windows::Win32::{
        Foundation,
        Foundation::{HANDLE, HINSTANCE, HWND, MAX_PATH, LPARAM, WPARAM, BOOL},
        System::{Diagnostics::Debug, Memory, ProcessStatus, Threading},UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, SendMessageW, WM_LBUTTONDOWN,
            WM_LBUTTONUP, WNDENUMPROC,
        },
    },
};

pub static HANDLER: Lazy<Handler> = Lazy::new(Handler::init);

#[derive(Debug)]
pub struct Handler {
    inner: HANDLE,
    hwnd: HWND,
}

// Not sure whether this is needed or not
impl Drop for Handler {
    fn drop(&mut self) {
        unsafe { Foundation::CloseHandle(self.inner) };
    }
}

impl Handler {
    fn init() -> Self {
        let processes = [0u32; 1024usize];
        let mut bytes_needed = 0u32;

        unsafe {
            ProcessStatus::K32EnumProcesses(
                processes.as_ptr() as _,
                mem::size_of_val(&processes) as _,
                &mut bytes_needed,
            );
        }

        for process in &processes[..bytes_needed as usize / mem::size_of::<u32>()] {
            let mut name = [0u16; MAX_PATH as _];
            let handle = match unsafe {
                Threading::OpenProcess(
                    Threading::PROCESS_QUERY_INFORMATION
                        | Threading::PROCESS_VM_OPERATION
                        | Threading::PROCESS_VM_READ
                        | Threading::PROCESS_VM_WRITE,
                    false,
                    *process,
                )
            } {
                Ok(ph) => ph,
                Err(_) => continue,
            };

            unsafe { ProcessStatus::K32GetModuleFileNameExW(handle, None, &mut name) };

            if !String::from_utf16_lossy(&name).contains("SpaceEngine.exe") {
                continue;
            }

            static WINDOW_HWND: OnceLock<HWND> = OnceLock::new();

            unsafe extern "system" fn enum_window(hwnd: HWND, _: LPARAM) -> BOOL {
                let length = GetWindowTextLengthW(hwnd);
                let mut bytes = vec![0u16; length as usize];
    
                GetWindowTextW(hwnd, &mut bytes);
    
                if OsString::from_wide(&bytes)
                    .into_string()
                    .unwrap()
                    .contains("SpaceEngine")
                {
                    WINDOW_HWND.set(hwnd);
                }
    
                return BOOL::from(true);
            }
    
            unsafe {
                EnumWindows(Some(enum_window), LPARAM(0isize)).unwrap();
            }

            return Self { inner: handle,             hwnd: *WINDOW_HWND.get().unwrap(), };
        }

        panic!("failed to find process: SpaceEngine.exe, maybe try opening it!");
    }

    #[must_use]
    pub fn exe(&self) -> PathBuf {
        let mut exe = [0u16; MAX_PATH as _];

        unsafe { ProcessStatus::K32GetModuleFileNameExW(self.inner, None, &mut exe) };

        PathBuf::from(String::from_utf16_lossy(&exe).replace('\0', ""))
    }

    #[must_use]
    pub fn base(&self) -> usize {
        self.base_of("SpaceEngine.exe")
    }

    #[must_use]
    pub fn base_of(&self, module_name: impl AsRef<str>) -> usize {
        let modules = [0usize; 1024usize];
        let mut bytes_needed = 0u32;

        unsafe {
            ProcessStatus::K32EnumProcessModules(
                self.inner,
                modules.as_ptr() as _,
                mem::size_of_val(&modules) as _,
                &mut bytes_needed,
            );
        }

        // This should convert both a String and &str to &str
        let module_name = module_name.as_ref();

        for module in &modules[..bytes_needed as usize / mem::size_of::<u32>()] {
            let mut name = [0u16; MAX_PATH as _];

            unsafe {
                ProcessStatus::K32GetModuleBaseNameW(
                    self.inner,
                    HINSTANCE(*module as _),
                    &mut name,
                );
            }

            if String::from_utf16_lossy(&name).contains(module_name) {
                return *module;
            }
        }

        panic!("failed to find module: {module_name}");
    }

    #[must_use]
    pub fn read_bytes(&self, base: usize, size: usize) -> Vec<u8> {
        let buffer = vec![0u8; size];

        unsafe {
            if !Debug::ReadProcessMemory(
                self.inner,
                base as _,
                buffer.as_ptr() as _,
                buffer.len(),
                None,
            )
            .as_bool()
            {
                panic!(
                    "failed to read bytes: {base:x}, {size:x}, {}",
                    Error::last_os_error()
                )
            }
        }

        buffer
    }

    /// Convenience function to call `read_bytes` with any type implementing
    /// `Pod`, rather than `Vec<u8>`.
    #[must_use]
    pub fn read<T: Pod>(&self, base: usize) -> T {
        *bytemuck::from_bytes::<T>(&self.read_bytes(base, mem::size_of::<T>()))
    }

    pub fn write_bytes(&self, base: usize, buffer: &[u8]) {
        let mut old_protection = Memory::PAGE_PROTECTION_FLAGS(0u32);

        unsafe {
            if !Memory::VirtualProtectEx(
                self.inner,
                base as _,
                buffer.len(),
                Memory::PAGE_EXECUTE_READWRITE,
                &mut old_protection,
            )
            .as_bool()
            {
                // Pretty sure I don't need to duplicate this to the other 2 functions here.
                panic!(
                    "failed to write bytes: {base:x}, {buffer:x?}, {}",
                    Error::last_os_error()
                )
            }

            Debug::WriteProcessMemory(
                self.inner,
                base as _,
                buffer.as_ptr().cast(),
                buffer.len(),
                None,
            );

            Memory::VirtualProtectEx(
                self.inner,
                base as _,
                buffer.len(),
                old_protection,
                &mut old_protection,
            );

            // This is entirely useless if some variable's being modified instead of
            // executable code, but I don't care.
            Debug::FlushInstructionCache(self.inner, Some(base as _), buffer.len());
        }
    }

    /// Convenience function to call `write_bytes` with any type implementing
    /// `Pod`, rather than `&[u8]`.
    pub fn write<T: Pod>(&self, buffer: T, base: usize) {
        self.write_bytes(base, bytemuck::bytes_of(&buffer));
    }

    /// Create and run an SE script.
    pub fn run_script(&self, name: impl AsRef<str>, buffer: impl AsRef<str>) {
        let path = env::current_dir().unwrap().join(name.as_ref());

        // Write bytes to script
        File::create(path.clone())
            .unwrap()
            .write_all(buffer.as_ref().as_bytes())
            .unwrap();

        Command::new(self.exe()).arg(path).spawn().unwrap();
    }

        /// Click by sending a message.
        pub fn click(&self, x: i32, y: i32) {
            unsafe {
                SendMessageW(
                    self.hwnd,
                    WM_LBUTTONDOWN,
                    WPARAM(0usize),
                    LPARAM(isize::overflowing_shl(y as isize, 16).0 | (x & 0xFFFF) as isize),
                )
            };
    
            unsafe {
                SendMessageW(
                    self.hwnd,
                    WM_LBUTTONUP,
                    WPARAM(0usize),
                    LPARAM(isize::overflowing_shl(y as isize, 16).0 | (x & 0xFFFF) as isize),
                )
            };
        }
}
