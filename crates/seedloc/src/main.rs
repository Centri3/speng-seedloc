use dll_syringe::process::OwnedProcess;
use dll_syringe::Syringe;
use std::env::current_exe;

fn main() {
    let se = OwnedProcess::find_first_by_name("SpaceEngine.exe").expect("Where is SE ):");
    let syringe = Syringe::for_process(se);

    syringe
        .inject(current_exe().unwrap().with_file_name("seedloc_dll.dll"))
        .expect("Could not inject seedloc DLL");
}
