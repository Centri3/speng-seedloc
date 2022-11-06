use std::mem;
use sysinfo::{PidExt, ProcessExt, System, SystemExt};
use windows::Win32::{
    Foundation::{self, HANDLE},
    System::{ProcessStatus, Threading},
};

#[derive(Clone, Copy)]
pub struct Handler {
    pub handle: HANDLE,
    pub base_address: usize,
}

impl Handler {
    pub fn new(sys: &mut System) -> Self {
        sys.refresh_all();

        let handle = match unsafe {
            Threading::OpenProcess(
                Threading::PROCESS_QUERY_INFORMATION
                    | Threading::PROCESS_VM_OPERATION
                    | Threading::PROCESS_VM_READ
                    | Threading::PROCESS_VM_WRITE,
                false,
                match sys.processes_by_exact_name("SpaceEngine.exe").nth(0usize) {
                    Some(pc) => pc.pid().as_u32(),
                    None => todo!(),
                },
            )
        } {
            Ok(ph) => ph,
            Err(_) => todo!(),
        };

        let modules = vec![0usize; 1024usize];

        unsafe {
            ProcessStatus::K32EnumProcessModules(
                handle,
                modules.as_ptr() as _,
                mem::size_of::<[usize; 1024usize]>() as _,
                &mut 0u32,
            )
        };

        let base_address = modules[0usize];

        Self {
            handle,
            base_address,
        }
    }

    pub fn close(&self) {
        unsafe { Foundation::CloseHandle(self.handle) };
    }
}
