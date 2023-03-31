mod seed;

use retour::static_detour;
use std::arch::asm;
use std::io::Write;
use std::mem::size_of;
use std::mem::transmute;
use std::ptr::addr_of_mut;
use std::thread;
use windows::Win32::System::ProcessStatus::K32EnumProcessModules;
use windows::Win32::System::Threading::GetCurrentProcess;

static_detour! {
    static Test: fn(usize, usize, *mut i32, i32);
}

#[no_mangle]
extern "system" fn DllMain(_: isize, reason: u32, _: usize) -> bool {
    if reason == 1u32 {
        thread::spawn(main);
    }

    true
}

static mut BASE: usize = 0x0usize;

fn main() {
    let base = unsafe {
        let mut temp = [0usize];

        K32EnumProcessModules(
            GetCurrentProcess(),
            addr_of_mut!(temp).cast(),
            size_of::<[usize; 1usize]>() as u32,
            &mut 0u32,
        );

        temp[0usize]
    };

    unsafe { BASE = base };

    unsafe {
        Test.initialize(transmute(base + 0x503480usize), hook)
            .unwrap();
        Test.enable().unwrap();
    }
}

fn hook(param_1: usize, param_2: usize, param_3: *mut i32, param_4: i32) {
    Test.call(param_1, param_2, param_3, param_4);

    let mut rbx: usize;
    unsafe { asm!("mov {},rbx", out(reg) rbx) };
    let rbx = rbx as *const ();

    let sector = unsafe { rbx.cast::<u8>().add(0x98usize).cast::<u32>().read() };
    let level = unsafe { rbx.cast::<u8>().add(0x9Cusize).cast::<u32>().read() };
    let block = unsafe { rbx.cast::<u8>().add(0xA0usize).cast::<u32>().read() };

    let stack_coords_stuff = (unsafe { BASE } + 0x19AA268usize) as *const ();
    let num_stars = unsafe { rbx.cast::<u8>().add(0xD8usize).cast::<u32>().read() };
    let stars = unsafe { rbx.cast::<u8>().add(0xDCusize).cast::<usize>().read() };

    if stars == 0usize || num_stars == 0u32 {
        return;
    }

    let stars = stars as *const ();

    // A for loop breaks SE for some reason
    let mut i = 0u32;
    loop {
        let mut log = std::fs::File::options()
            .append(true)
            .create(true)
            .open("seedlochaha.log")
            .unwrap();

        if i >= num_stars {
            return;
        }

        // +24 = system's main star
        // +30 = x
        // +34 = y
        // +38 = z
        let main_star = unsafe {
            stars
                .cast::<u8>()
                .add(0x3Cusize * i as usize + 0x24usize)
                .cast::<u16>()
                .read()
        };

        let (star_x, star_y, star_z) = unsafe {
            stars
                .cast::<u8>()
                .add(0x3Cusize * i as usize + 0x30usize)
                .cast::<(f32, f32, f32)>()
                .read()
        };

        let (sector_x, sector_y, sector_z) = unsafe {
            rbx.cast::<u8>()
                .add(0x60usize)
                .cast::<(f64, f64, f64)>()
                .read()
        };

        let (mut x, mut y, mut z) = (
            (star_x as f64 + sector_x),
            (star_y as f64 + sector_y),
            (star_z as f64 + sector_z),
        );

        let x_double = y * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x110usize)
                .cast::<f64>()
                .read()
        } + x * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0xF8usize)
                .cast::<f64>()
                .read()
        } + z * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x128usize)
                .cast::<f64>()
                .read()
        };

        let y_double = x * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x100usize)
                .cast::<f64>()
                .read()
        } + y * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x118usize)
                .cast::<f64>()
                .read()
        } + z * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x130usize)
                .cast::<f64>()
                .read()
        };

        let z_double = x * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x108usize)
                .cast::<f64>()
                .read()
        } + y * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x120usize)
                .cast::<f64>()
                .read()
        } + z * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x138usize)
                .cast::<f64>()
                .read()
        };

        let fun_140410ae0 =
            unsafe { transmute::<_, unsafe extern "C" fn(f64) -> u128>(BASE + 0x410AE0usize) };

        let x = unsafe { fun_140410ae0(x_double) };
        let y = unsafe { fun_140410ae0(y_double) };
        let z = unsafe { fun_140410ae0(z_double) };

        let seed = seed::seed((x, y, z));

        if i == 0 {
            writeln!(
                log,
                "RS (CURRENTGALAXY)-{sector}-{level}-{block}-{i}\nX: {x:X}\nY: {y:X}\nZ: \
                 {z:X}\nSEED: {seed:X}"
            )
            .unwrap();
        }

        i += 1u32;
    }
}

// fn fun_140410ae0(param_1: f64) -> u128 {
//     let abs = param_1.abs();
//
//     let d_var4 = f64::floor(abs * 1.52587890625e-05f64);
//     let mut d_var1 = abs - (d_var4 as i64 & 0xffffffffi64) as f64 *
// 65536.0f64;     let mut u_var2 = (d_var1 * 65536.0f64) as u64;
//     let mut u_var3 = (d_var4 as i64) << 0x20usize | u_var2 as i64 &
// 0xffffffffi64;     d_var1 -= (u_var2 & 0xffffffffu64) as f64 *
// 1.52587890625e-05f64;     u_var2 = (d_var1 * 2.0f64.powi(48i32)) as u64;
//     let mut u_var2 = ((d_var1 - (u_var2 & 0xffffffffu64) as f64 *
// 3.552713678800501e-15f64)
//         * 1.20892581961462e+24f64) as i64
//         & 0xffffffffi64
//         | (u_var2 as i64) << 0x20usize;
//
//     if param_1.is_sign_negative() {
//         u_var3 = !u_var3;
//         u_var2 = !u_var2 + 1i64;

//         if u_var2 == 0i64 {
//             u_var3 += 1i64;
//         }
//     }
//
//     u128::from_le_bytes(
//         [u_var3.to_le_bytes(), u_var2.to_le_bytes()]
//             .concat()
//             .try_into()
//             .unwrap(),
//     )
// }
