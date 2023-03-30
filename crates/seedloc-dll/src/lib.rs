use retour::static_detour;
use std::arch::asm;
use std::io::Write;
use std::mem::size_of;
use std::mem::transmute;
use std::ptr::addr_of_mut;
use std::thread;
use windows::Win32::System::Diagnostics::Debug::DebugBreak;
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

    if stars == 0usize {
        return;
    }

    let stars = stars as *const ();

    // A for loop breaks SE for some reason
    let mut i = 0u32;
    loop {
        let mut log = std::fs::File::create(format!("S {sector}-{level}-{block}")).unwrap();

        i += 1u32;
        if i >= num_stars {
            break;
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

        // TODO: It doesn't get this correctly?
        let (star_x, star_y, star_z) = unsafe {
            stars
                .cast::<u8>()
                .add(0x3Cusize * i as usize + 0x30usize)
                .cast::<(f32, f32, f32)>()
                .read()
        };

        // TODO: These are actually f64s...
        let (sector_x, sector_y, sector_z) = unsafe {
            rbx.cast::<u8>()
                .add(0x60usize)
                .cast::<(f32, f32, f32)>()
                .read()
        };

        let (mut x, mut y, mut z) = (
            (star_x + sector_x) as f64,
            (star_y + sector_y) as f64,
            (star_z + sector_z) as f64,
        );

        writeln!(
            log,
            "star_x: {star_x}, star_y: {star_y}, star_z: {star_z}, sector_x: {sector_x}, \
             sector_y: {sector_y}, sector_z: {sector_z}"
        );

        let d_var13 = y * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x110usize)
                .cast::<f32>()
                .read() as f64
        } + x * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0xF8usize)
                .cast::<f32>()
                .read() as f64
        } + z * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x128usize)
                .cast::<f32>()
                .read() as f64
        };

        let d_var3 = x * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x100usize)
                .cast::<f32>()
                .read() as f64
        } + y * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x118usize)
                .cast::<f32>()
                .read() as f64
        } + z * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x130usize)
                .cast::<f32>()
                .read() as f64
        };

        let xx = y * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x108usize)
                .cast::<f32>()
                .read() as f64
        } + x * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x120usize)
                .cast::<f32>()
                .read() as f64
        } + z * unsafe {
            stack_coords_stuff
                .cast::<u8>()
                .add(0x138usize)
                .cast::<f32>()
                .read() as f64
        };

        let x = fun_140410ae0(d_var13);
        let y = fun_140410ae0(d_var3);
        let z = fun_140410ae0(xx);

        writeln!(log, "X: {x:X}\nY: {y:X}\nZ: {z:X}");

        break;
    }
}

fn fun_140410ae0(param_1: f64) -> u128 {
    let abs = param_1.abs();

    let d_var4 = f64::floor(abs * 1.52587890625e-05f64);
    let mut d_var1 = abs - (d_var4 as i64 & 0xffffffffi64) as f64 * 65536.0f64;
    let mut u_var2 = (d_var1 * 65536.0f64) as u64;
    let mut u_var3 = (d_var4 as i64) << 0x20usize | u_var2 as i64 & 0xffffffffi64;
    d_var1 -= (u_var2 & 0xffffffffu64) as f64 * 1.52587890625e-05f64;
    u_var2 = (d_var1 * 2.0f64.powi(48i32)) as u64;
    let mut u_var2 = ((d_var1 - (u_var2 & 0xffffffffu64) as f64 * 3.552713678800501e-15f64) as f64
        * 1.208925819614629e+24f64) as i64
        & 0xffffffffi64
        | (u_var2 as i64) << 0x20usize;

    if param_1.is_sign_negative() {
        u_var3 = !u_var3;
        u_var2 = !u_var2 + 1i64;

        if u_var2 == 0i64 {
            u_var3 += 1i64;
        }
    }

    u128::from_le_bytes(
        [u_var3.to_le_bytes(), u_var2.to_le_bytes()]
            .concat()
            .try_into()
            .unwrap(),
    )
}
