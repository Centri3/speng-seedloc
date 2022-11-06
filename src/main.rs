mod handler;
mod seed;

use enigo::{Enigo, MouseButton, MouseControllable};
use handler::Handler;
use rand::{rngs::ThreadRng, Rng};
use std::{env, fs::File, io::Write, mem, process::Command, thread, time::Duration};
use sysinfo::{ProcessExt, System, SystemExt};
use windows::Win32::System::Diagnostics::Debug;

const SEEDS: [i32; 4usize] = [-1175671629i32, 299793970i32, -808771962i32, -573742822i32];

const GLOBAL_COORDS: usize = 0xe27740usize;
const LOCAL_COORDS: usize = 0x1984e54usize;
const SYSTEMS_FOUND: usize = 0x1022dc8usize;
const SEARCH_BUTTON_COORDS: usize = 0x1024728usize;
const FILTER_SORT_COORDS: usize = 0x1026720usize;
const SELECTED_OBJECT_CODE: usize = 0x19a8ab0usize;
const SELECTED_OBJECT_POINTER: usize = 0x19a8b30usize;

fn get_global(handler: Handler) -> (u64, u64, u64) {
    let x = unsafe {
        let buffer = [0u8; 8usize];

        Debug::ReadProcessMemory(
            handler.handle,
            (handler.base_address + GLOBAL_COORDS) as _,
            buffer.as_ptr() as _,
            mem::size_of::<[u8; 8usize]>(),
            None,
        );

        u64::from_le_bytes(buffer)
    };

    let y = unsafe {
        let buffer = [0u8; 8usize];

        Debug::ReadProcessMemory(
            handler.handle,
            (handler.base_address + GLOBAL_COORDS + 0x10) as _,
            buffer.as_ptr() as _,
            mem::size_of::<[u8; 8usize]>(),
            None,
        );

        u64::from_le_bytes(buffer)
    };

    let z = unsafe {
        let buffer = [0u8; 8usize];

        Debug::ReadProcessMemory(
            handler.handle,
            (handler.base_address + GLOBAL_COORDS + 0x20) as _,
            buffer.as_ptr() as _,
            mem::size_of::<[u8; 8usize]>(),
            None,
        );

        u64::from_le_bytes(buffer)
    };

    (x, y, z)
}

fn create_run_file(name: &str, contents: &str, sys: &mut System) {
    sys.refresh_all();

    let mut file = File::create(name).unwrap();
    writeln!(file, "{}", contents).unwrap();

    let mut path = env::current_dir().unwrap();
    path.push(name);

    Command::new(
        sys.processes_by_exact_name("SpaceEngine.exe")
            .nth(0usize)
            .unwrap()
            .exe(),
    )
    .arg(path)
    .spawn()
    .unwrap();
}

fn goto_galaxy(handler: Handler, mut rng: &mut ThreadRng, mut sys: &mut System) {
    loop {
        let octree_level = rng.gen_range(0u32..=4u32);
        let octree_block = rng.gen_range(0u32..=8u32.pow(octree_level));
        let number = rng.gen_range(0u32..=2500u32);

        create_run_file("select_rg_397.se", "Select \"RG 0-3-397-1581\"", &mut sys);

        thread::sleep(Duration::from_millis(500u64));

        unsafe {
            Debug::WriteProcessMemory(
                handler.handle,
                (handler.base_address + SELECTED_OBJECT_CODE + 0x4) as _,
                octree_level.to_le_bytes().as_ptr() as _,
                mem::size_of::<u32>() as _,
                None,
            );

            Debug::WriteProcessMemory(
                handler.handle,
                (handler.base_address + SELECTED_OBJECT_CODE + 0x8) as _,
                octree_block.to_le_bytes().as_ptr() as _,
                mem::size_of::<u32>() as _,
                None,
            );

            Debug::WriteProcessMemory(
                handler.handle,
                (handler.base_address + SELECTED_OBJECT_CODE + 0x10) as _,
                number.to_le_bytes().as_ptr() as _,
                mem::size_of::<u32>() as _,
                None,
            );
        };

        thread::sleep(Duration::from_millis(100u64));

        let buffer = [0u8; 4usize];

        unsafe {
            Debug::ReadProcessMemory(
                handler.handle,
                (handler.base_address + SELECTED_OBJECT_CODE + 0x1D8) as _,
                buffer.as_ptr() as _,
                mem::size_of::<[u8; 4usize]>(),
                None,
            )
        };

        if u32::from_le_bytes(buffer) != 0 {
            continue;
        }

        thread::sleep(Duration::from_millis(100u64));

        let buffer = [0u8; 8usize];

        unsafe {
            Debug::ReadProcessMemory(
                handler.handle,
                (handler.base_address + SELECTED_OBJECT_POINTER) as _,
                buffer.as_ptr() as _,
                mem::size_of::<[u8; 8usize]>(),
                None,
            )
        };

        let selected_object_address = usize::from_le_bytes(buffer);

        let buffer = [0u8; 4usize];

        unsafe {
            Debug::ReadProcessMemory(
                handler.handle,
                (selected_object_address + 0x20) as _,
                buffer.as_ptr() as _,
                mem::size_of::<[u8; 4usize]>(),
                None,
            )
        };

        if f32::from_le_bytes(buffer) != 50000.0f32 {
            continue;
        }

        let lon = rng.gen_range(-180.0f64..180.0f64);
        let dist_rad = rng.gen_range(0.0f64..0.25f64);

        create_run_file(
            "goto_galaxy.se",
            format!("Goto {{ Lat {} Time 0 }}", lon).as_str(),
            &mut sys,
        );

        thread::sleep(Duration::from_millis(100u64));

        create_run_file("center_galaxy.se", "Goto { Lon 90 Time 0 }", &mut sys);

        thread::sleep(Duration::from_millis(100u64));

        create_run_file(
            "randomize_galaxy.se",
            format!("Goto {{ DistRad {} Time 0 }}", dist_rad).as_str(),
            sys,
        );

        break;
    }
}

fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let handler = Handler::new(&mut sys);
    let mut rng = rand::thread_rng();

    loop {
        goto_galaxy(handler, &mut rng, &mut sys);

        create_run_file("select_sol.se", "Select \"Solar System\"", &mut sys);

        create_run_file("follow.se", "Follow", &mut sys);

        thread::sleep(Duration::from_millis(100u64));

        let global = get_global(handler);

        for _ in 1u32..10000000u32 {
            let x = rng.gen_range(global.0 - 0x1329999u64..global.0 + 0x1329999u64);
            let y = rng.gen_range(global.1 - 0x1329999u64..global.1 + 0x1329999u64);
            let z = rng.gen_range(global.2 - 0x1329999u64..global.2 + 0x1329999u64);

            let seed = seed::seed((x, y, z));

            if SEEDS.contains(&seed) {
                let coords = unsafe {
                    Debug::WriteProcessMemory(
                        handler.handle,
                        (handler.base_address + GLOBAL_COORDS) as _,
                        x.to_le_bytes().as_ptr() as _,
                        mem::size_of::<u64>(),
                        None,
                    );

                    Debug::WriteProcessMemory(
                        handler.handle,
                        (handler.base_address + LOCAL_COORDS + 0x10) as _,
                        y.to_le_bytes().as_ptr() as _,
                        mem::size_of::<u64>(),
                        None,
                    );

                    Debug::WriteProcessMemory(
                        handler.handle,
                        (handler.base_address + LOCAL_COORDS + 0x20) as _,
                        z.to_le_bytes().as_ptr() as _,
                        mem::size_of::<u64>(),
                        None,
                    );

                    let buffer = [0u8; 4usize];

                    Debug::ReadProcessMemory(
                        handler.handle,
                        (handler.base_address + SEARCH_BUTTON_COORDS) as _,
                        buffer.as_ptr() as _,
                        mem::size_of::<f32>(),
                        None,
                    );

                    let x = f32::from_le_bytes(buffer);

                    let buffer = [0u8; 4usize];

                    Debug::ReadProcessMemory(
                        handler.handle,
                        (handler.base_address + SEARCH_BUTTON_COORDS + 0x4) as _,
                        buffer.as_ptr() as _,
                        mem::size_of::<f32>(),
                        None,
                    );

                    let y = f32::from_le_bytes(buffer);

                    (x, y)
                };

                let mut enigo = Enigo::new();

                enigo.mouse_move_to(coords.0 as i32 + 10i32, coords.1 as i32 + 30i32);

                thread::sleep(Duration::from_millis(1000u64));

                let coords = unsafe {
                    let buffer = [0u8; 4usize];

                    Debug::ReadProcessMemory(
                        handler.handle,
                        (handler.base_address + FILTER_SORT_COORDS) as _,
                        buffer.as_ptr() as _,
                        mem::size_of::<f32>(),
                        None,
                    );

                    let x = f32::from_le_bytes(buffer);

                    let buffer = [0u8; 4usize];

                    Debug::ReadProcessMemory(
                        handler.handle,
                        (handler.base_address + FILTER_SORT_COORDS + 0x4) as _,
                        buffer.as_ptr() as _,
                        mem::size_of::<f32>(),
                        None,
                    );

                    let y = f32::from_le_bytes(buffer);

                    (x, y)
                };

                enigo.mouse_move_to(coords.0 as i32 + 10i32, coords.1 as i32 + 30i32);

                let systems_found = unsafe {
                    let buffer = [0u8; 4usize];

                    Debug::ReadProcessMemory(
                        handler.handle,
                        (handler.base_address + SYSTEMS_FOUND) as _,
                        buffer.as_ptr() as _,
                        mem::size_of::<u32>(),
                        None,
                    );

                    u32::from_le_bytes(buffer)
                };

                if systems_found != 0u32 {
                    enigo.mouse_move_relative(0i32, 0x25i32);

                    enigo.mouse_click(MouseButton::Left);

                    thread::sleep(Duration::from_millis(100u64));

                    let system_seed = unsafe {
                        let buffer = [0u8; 8usize];

                        Debug::ReadProcessMemory(
                            handler.handle,
                            (handler.base_address + 0x19a8b38usize) as _,
                            buffer.as_ptr() as _,
                            8usize,
                            None,
                        );

                        let address = usize::from_le_bytes(buffer);
                        let buffer = [0u8; 4usize];

                        Debug::ReadProcessMemory(
                            handler.handle,
                            (address + 0x170usize) as _,
                            buffer.as_ptr() as _,
                            4usize,
                            None,
                        );

                        i32::from_le_bytes(buffer)
                    };

                    if seed == system_seed {
                        create_run_file(
                            "print_to_log.se",
                            format!("Log \"Found seed: {}\"\nPrintNames true", seed).as_str(),
                            &mut sys,
                        );
                    }
                }
            }
        }
    }
}
