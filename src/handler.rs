use bytemuck::Pod;
use once_cell::sync::Lazy;
use std::env;
use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::mem;
use std::path::PathBuf;
use std::process::Command;
use sysinfo::PidExt;
use sysinfo::ProcessExt;
use sysinfo::System;
use sysinfo::SystemExt;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::Diagnostics::Debug;
use windows::Win32::System::Memory;
use windows::Win32::System::ProcessStatus;
use windows::Win32::System::Threading::GetProcessId;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_QUERY_INFORMATION;
use windows::Win32::System::Threading::PROCESS_VM_OPERATION;
use windows::Win32::System::Threading::PROCESS_VM_READ;
use windows::Win32::System::Threading::PROCESS_VM_WRITE;

pub fn handler() -> &'static Handler {
    static _SE_HANDLER: Lazy<Handler> = Lazy::new(|| {
        let mut sys = System::new_all();
        sys.refresh_all();

        let pid = sys
            .processes_by_exact_name("SpaceEngine.exe")
            .next()
            .expect("Where is SE ):")
            .pid()
            .as_u32();

        unsafe {
            Handler(
                OpenProcess(
                    PROCESS_QUERY_INFORMATION
                        | PROCESS_VM_OPERATION
                        | PROCESS_VM_READ
                        | PROCESS_VM_WRITE,
                    false,
                    pid,
                )
                .unwrap(),
            )
        }
    });

    &_SE_HANDLER
}

pub struct Handler(HANDLE);

impl Drop for Handler {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0).ok().expect("Failed to close handle") };
    }
}

impl Handler {
    #[must_use]
    pub fn pid(&self) -> u32 {
        unsafe { GetProcessId(self.0) }
    }

    #[must_use]
    pub fn exe(&self) -> PathBuf {
        let mut exe = [0u16; MAX_PATH as _];

        unsafe { ProcessStatus::K32GetModuleFileNameExW(self.0, None, &mut exe) };

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
                self.0,
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
                ProcessStatus::K32GetModuleBaseNameW(self.0, HINSTANCE(*module as _), &mut name);
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
                self.0,
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
                self.0,
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
                self.0,
                base as _,
                buffer.as_ptr().cast(),
                buffer.len(),
                None,
            );

            Memory::VirtualProtectEx(
                self.0,
                base as _,
                buffer.len(),
                old_protection,
                &mut old_protection,
            );

            // This is entirely useless if some variable's being modified instead of
            // executable code, but I don't care.
            Debug::FlushInstructionCache(self.0, Some(base as _), buffer.len());
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
}
