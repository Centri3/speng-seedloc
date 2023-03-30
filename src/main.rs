mod handler;

use aligned::Aligned;
use aligned::A16;
use fixed::types::I48F80;
use handler::handler;
use rand::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::mem::size_of;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use windows::imp::GetProcAddress;
use windows::s;
use windows::Win32::Foundation::DBG_CONTINUE;
use windows::Win32::Foundation::DBG_EXCEPTION_HANDLED;
use windows::Win32::Foundation::DBG_EXCEPTION_NOT_HANDLED;
use windows::Win32::Foundation::EXCEPTION_SINGLE_STEP;
use windows::Win32::System::Diagnostics::Debug::ContinueDebugEvent;
use windows::Win32::System::Diagnostics::Debug::DebugActiveProcess;
use windows::Win32::System::Diagnostics::Debug::DebugActiveProcessStop;
use windows::Win32::System::Diagnostics::Debug::DebugSetProcessKillOnExit;
use windows::Win32::System::Diagnostics::Debug::GetThreadContext;
use windows::Win32::System::Diagnostics::Debug::SetThreadContext;
use windows::Win32::System::Diagnostics::Debug::WaitForDebugEvent;
use windows::Win32::System::Diagnostics::Debug::CONTEXT;
use windows::Win32::System::Diagnostics::Debug::DEBUG_EVENT;
use windows::Win32::System::Diagnostics::Debug::EXCEPTION_DEBUG_EVENT;
use windows::Win32::System::Diagnostics::ToolHelp::CreateToolhelp32Snapshot;
use windows::Win32::System::Diagnostics::ToolHelp::Thread32Next;
use windows::Win32::System::Diagnostics::ToolHelp::TH32CS_SNAPTHREAD;
use windows::Win32::System::Diagnostics::ToolHelp::THREADENTRY32;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::Threading::OpenThread;
use windows::Win32::System::Threading::THREAD_GET_CONTEXT;
use windows::Win32::System::Threading::THREAD_SET_CONTEXT;

// Address to the code of the selected object.
// Example: RS 0-3-397-1581-20880-7-556321-30 A3. Go visit it yourself! (:
const SELECTED_OBJECT_CODE: usize = 0x19A9E40usize;
// Pointer to the parameters of the selected object.
const SELECTED_OBJECT_POINTER: usize = 0x19A9EC0usize;
const GALAXY_TYPE: usize = 0x8usize;
const GALAXY_SIZE: usize = 0x20usize;

fn main() {
    let x = I48F80::from_num(-360.23f64);
    for i in x.to_le_bytes() {
        print!("{:X}", i);
    }
    println!();

    panic!();

    let seeds_txt = fs::read_to_string("seeds.txt").unwrap();
    // What the fuck
    let seeds = seeds_txt
        .lines()
        .map(|l| {
            // This will split at whitespace, can't use .split_whitespace() because
            // .as_str() isn't stable! <https://github.com/rust-lang/rust/issues/77998>
            let line = l.split_once(' ').unwrap();

            // Isolate both the seed and types of stars
            let seed = line.0;
            let types = line.1;

            (
                seed.parse::<i32>().unwrap(),
                // Since we used .split_once() earlier, we can .split() again to get an iterator
                // over each star type. Spaghetti, but it works!
                types
                    .split(' ')
                    .map(|t| t.parse::<u16>().unwrap())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<(_, Vec<_>)>>();

    let handler = handler();
    let mut rng = thread_rng();
    let base = handler.base();

    // This will prevent SE from focusing window when running scripts
    handler.write_bytes(
        unsafe {
            GetProcAddress(
                GetModuleHandleA(s!("user32.dll")).unwrap().0,
                s!("SetForegroundWindow"),
            ) as usize
        },
        &[0xC3u8],
    );

    // Select RG 0-3-397-1581, this is so we can reset the code of the currently
    // selected object. If we don't do this, it'll select nothing.
    handler.run_script("select_rg_397.se", r#"Select "RG 0-3-397-1581""#);

    thread::sleep(Duration::from_secs(1u64));

    let mut log = File::create("seedloc.log").unwrap();
    let mut already_seen = HashMap::new();

    loop {
        let g_level = rng.gen_range(1u32..9u32);
        let g_block = rng.gen_range(0u32..8u32.pow(g_level));
        let gal_num = rng.gen_range(0u32..2500u32);

        handler.write(g_level, base + SELECTED_OBJECT_CODE + 0x4);
        handler.write(g_block, base + SELECTED_OBJECT_CODE + 0x8);
        handler.write(gal_num, base + SELECTED_OBJECT_CODE + 0x10);

        thread::sleep(Duration::from_millis(240u64));

        let selected_galaxy = handler.read::<usize>(base + SELECTED_OBJECT_POINTER);

        // Galaxy does not exist or our code is too fast
        if handler.read::<usize>(base + SELECTED_OBJECT_POINTER) == 0usize || {
            (1u32..=8u32).contains(&handler.read::<u32>(selected_galaxy + GALAXY_TYPE))
                || handler.read::<u32>(selected_galaxy + GALAXY_TYPE) == 16u32
                || handler.read::<f32>(selected_galaxy + GALAXY_SIZE) != 50000.0f32
        } {
            continue;
        }

        handler.run_script(
            "goto.se",
            format!(
                "Goto {{ Lat {} Lon 90 Time 0 }}",
                rng.gen_range(0.0f32..360.0f32)
            ),
        );

        thread::sleep(Duration::from_secs(1u64));

        // DistRad and Lat/Lon don't work together, for some reason
        handler.run_script("goto_closer.se", "Goto { DistRad 0.4 Time 0 }");

        thread::sleep(Duration::from_millis(240u64));

        // "Orbit" the galaxy. AKA move around
        handler.run_script("orbit_galaxy.se", "Orbit { Axis (0 0 1) FadeTime 0 }");

        thread::sleep(Duration::from_millis(60u64));

        unsafe {
            DebugActiveProcess(handler.pid());
            DebugSetProcessKillOnExit(false);
        }

        let snapshot =
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, handler.pid()).unwrap() };
        let mut entry = THREADENTRY32 {
            dwSize: size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };

        let mut threads = vec![];

        // Setup a breakpoint for every one of SE's threads
        while unsafe { Thread32Next(snapshot, &mut entry).as_bool() } {
            if entry.th32OwnerProcessID == handler.pid() {
                let hthread = unsafe {
                    OpenThread(
                        THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                        false,
                        entry.th32ThreadID,
                    )
                    .unwrap()
                };

                threads.push(hthread);
            }
        }

        for hthread in threads.iter().copied() {
            let mut ctx = Aligned::<A16, _>(CONTEXT::default());
            ctx.ContextFlags = 1048592u32;

            unsafe { GetThreadContext(hthread, &mut *ctx).unwrap() };

            // TOFINDTHIS: Find where Creating stars block "%s" is referenced and search for
            // test al,al below it
            ctx.Dr0 = base as u64 + 0x502484u64;
            ctx.Dr7 = 0x401u64;

            unsafe { SetThreadContext(hthread, &*ctx).unwrap() };
        }

        let start = Instant::now();
        let mut i = 0u32;
        let mut num_of_stars = 0u64;

        'a: loop {
            let mut dbg_event = DEBUG_EVENT::default();
            unsafe { WaitForDebugEvent(&mut dbg_event, u32::MAX) };

            if dbg_event.dwDebugEventCode == EXCEPTION_DEBUG_EVENT {
                let info = unsafe { dbg_event.u.Exception };

                if info.ExceptionRecord.ExceptionCode == EXCEPTION_SINGLE_STEP
                    && info.ExceptionRecord.ExceptionAddress as usize == base + 0x502484usize
                {
                    let mut ctx_debug = Aligned::<A16, _>(CONTEXT::default());
                    ctx_debug.ContextFlags = 1048592u32;

                    // wtf

                    unsafe {
                        GetThreadContext(
                            OpenThread(
                                THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                false,
                                dbg_event.dwThreadId,
                            )
                            .unwrap(),
                            &mut *ctx_debug,
                        );
                    }

                    // I don't know why I need to do this but if I don't then it crashes.
                    // looooooooooooooool
                    let mut ctx = Aligned::<A16, _>(CONTEXT::default());
                    ctx.ContextFlags = 1048592u32 | 0x2;

                    unsafe {
                        GetThreadContext(
                            OpenThread(
                                THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                false,
                                dbg_event.dwThreadId,
                            )
                            .unwrap(),
                            &mut *ctx,
                        );
                    }

                    let sector = handler.read::<u32>(ctx.Rbx as usize + 0x98usize);
                    let level = handler.read::<u32>(ctx.Rbx as usize + 0x9Cusize);
                    let block = handler.read::<u32>(ctx.Rbx as usize + 0xA0usize);

                    if already_seen
                        .insert((g_level, g_block, gal_num, sector, level, block), "seen!")
                        .is_none()
                    {
                        i += 1u32;

                        'b: {
                            // Just in case?
                            if ctx.Rbx < 10000u64 {
                                break 'b;
                            }

                            let num_stars = handler.read::<u32>(ctx.Rbx as usize + 0xD8usize);
                            let stars = handler.read::<usize>(ctx.Rbx as usize + 0xDCusize);

                            num_of_stars += num_stars as u64;

                            println!("{num_stars}");

                            // Stars do not exist??????
                            if stars == 0usize {
                                break 'b;
                            }

                            for i in 0u32..num_stars {
                                // +24 = system's main star
                                // +30 = x
                                // +34 = y
                                // +38 = z
                                if handler.read::<u16>(stars + i as usize * 0x3Cusize + 0x24usize)
                                    == 14996u16
                                {
                                    writeln!(log, "found G9.8!: RS 0-{g_level}-{g_block}-{gal_num}-{sector}-{level}-{block}-{i}").unwrap();
                                }
                            }

                            // println!("{stars:X}");
                        }
                    }

                    ctx_debug.Dr0 = 0x0u64;
                    ctx_debug.Dr7 = 0x0u64;

                    unsafe {
                        SetThreadContext(
                            OpenThread(
                                THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                false,
                                dbg_event.dwThreadId,
                            )
                            .unwrap(),
                            &*ctx_debug,
                        );
                    }

                    // TODO: CHANGE LATER
                    if i == 2500u32 || start.elapsed().as_secs_f64() > 30.0f64 {
                        let snapshot = unsafe {
                            CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, handler.pid()).unwrap()
                        };
                        let mut entry = THREADENTRY32 {
                            dwSize: size_of::<THREADENTRY32>() as u32,
                            ..Default::default()
                        };

                        while unsafe { Thread32Next(snapshot, &mut entry).as_bool() } {
                            if entry.th32OwnerProcessID == handler.pid() {
                                let hthread = unsafe {
                                    OpenThread(
                                        THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                        false,
                                        entry.th32ThreadID,
                                    )
                                    .unwrap()
                                };

                                let mut ctx = Aligned::<A16, _>(CONTEXT::default());
                                ctx.ContextFlags = 1048592u32;

                                unsafe { GetThreadContext(hthread, &mut *ctx).unwrap() };

                                ctx.Dr0 = 0x0u64;
                                ctx.Dr7 = 0x0u64;

                                unsafe { SetThreadContext(hthread, &*ctx).unwrap() };
                            }
                        }

                        println!("count: {num_of_stars}, elapsed: {}", start.elapsed().as_secs_f64());

                        println!("threads stopped");

                        unsafe {
                            ContinueDebugEvent(
                                dbg_event.dwProcessId,
                                dbg_event.dwThreadId,
                                DBG_EXCEPTION_HANDLED,
                            );
                        }

                        unsafe { DebugActiveProcessStop(dbg_event.dwProcessId) };

                        println!("leaving");

                        break 'a;
                    }

                    unsafe {
                        ContinueDebugEvent(
                            dbg_event.dwProcessId,
                            dbg_event.dwThreadId,
                            DBG_EXCEPTION_HANDLED,
                        );
                    }

                    println!("lol?");

                    ctx_debug.Dr0 = base as u64 + 0x502484u64;
                    ctx_debug.Dr7 = 0x401u64;

                    unsafe {
                        SetThreadContext(
                            OpenThread(
                                THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                false,
                                dbg_event.dwThreadId,
                            )
                            .unwrap(),
                            &*ctx_debug,
                        );
                    }

                    continue;
                }
                else {
                    unsafe {
                        ContinueDebugEvent(
                            dbg_event.dwProcessId,
                            dbg_event.dwThreadId,
                            DBG_EXCEPTION_NOT_HANDLED,
                        );
                    }

                    continue;
                }
            }

            unsafe {
                ContinueDebugEvent(dbg_event.dwProcessId, dbg_event.dwThreadId, DBG_CONTINUE);
            }
        }

        println!("left");

        handler.run_script("goto_far.se", "Goto { DistRad 100 Time 0 }");

        thread::sleep(Duration::from_secs(2u64));

        handler.run_script("stop_orbit_galaxy.se", "StopOrbit { FadeTime 0 }");

        thread::sleep(Duration::from_secs(1u64));
    }
}
