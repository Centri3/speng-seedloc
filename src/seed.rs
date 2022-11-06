#![allow(dead_code)]

// Ghidra's Internal Decompiler Functions

fn concat_44(x: u32, y: u32) -> u64 {
    let mut bytes = vec![];

    for byte in y.to_le_bytes().iter() {
        bytes.push(*byte);
    }

    for byte in x.to_le_bytes().iter() {
        bytes.push(*byte);
    }

    u64::from_le_bytes(bytes.try_into().unwrap())
}

// Decompiled by Ghidra, thanks!
fn hash(param_1: u64, param_2: u64) -> f64 {
    let u_var1: u64;
    let mut u_var2: u64;
    let mut u_var3: u64;
    let mut d_var4: f64;

    u_var1 = param_1;
    u_var3 = param_2;
    u_var2 = u_var1;

    if 0x7fffffffffffffff < u_var1 {
        u_var2 = !u_var1;
        u_var3 = !u_var3 + 1u64;
        if u_var3 == 0u64 {
            u_var2 += 1u64;
        }
    }

    d_var4 = (u_var3 >> 0x20u64) as f64 * 3.552713678800501e-15f64
        + (u_var3 & 0xffffffff) as f64 * 8.271806125530277e-25f64
        + (u_var2 & 0xffffffff) as f64 * 1.52587890625e-05f64
        + (u_var2 >> 0x20u64) as f64 * 65536.0f64;

    if 0x7fffffffffffffff < u_var1 {
        d_var4 = f64::from_le_bytes(
            (u64::from_le_bytes(d_var4.to_le_bytes()) ^ 0x8000000000000000u64).to_le_bytes(),
        );
    }

    d_var4
}

// Decompilated also by Ghidra, thanks again!
pub fn seed(coords: (u64, u64, u64)) -> i32 {
    loop {
        let mut d_var1: f64;
        let d_var2: f64;
        let d_var3: f64;
        let u_var4: u32;
        let u_var5: u32;
        let u_var6: u32;
        let u_var7: u32;
        let u_var8: u32;
        let u_var9: u32;
        let d_var10: f64;
        let d_var11: f64;
        let d_var12: f64;

        d_var10 = 100.0;
        d_var11 = 2147483647.0f64;
        d_var12 = 0.5f64;

        d_var1 = hash(coords.0, 0u64) * d_var10;
        u_var4 = (u64::from_le_bytes(d_var1.to_le_bytes())) as u32;
        u_var7 = (u64::from_le_bytes(d_var1.to_le_bytes()) >> 0x20u64) as u32;
        d_var1 = f64::floor(d_var1 / d_var11 * d_var12 + d_var12);
        d_var2 = (d_var1 + d_var1) * d_var11;

        d_var1 = hash(coords.1, 0u64) * d_var10;
        u_var5 = (u64::from_le_bytes(d_var1.to_le_bytes())) as u32;
        u_var8 = (u64::from_le_bytes(d_var1.to_le_bytes()) >> 0x20u64) as u32;
        d_var1 = f64::floor(d_var1 / d_var11 * d_var12 + d_var12);
        d_var3 = (d_var1 + d_var1) * d_var11;

        d_var1 = hash(coords.2, 0u64) * d_var10;
        u_var6 = (u64::from_le_bytes(d_var1.to_le_bytes())) as u32;
        u_var9 = (u64::from_le_bytes(d_var1.to_le_bytes()) >> 0x20u64) as u32;
        d_var1 = f64::floor(d_var1 / d_var11 * d_var12 + d_var12);

        return (f64::from_le_bytes(concat_44(u_var8, u_var5).to_le_bytes()) - d_var3) as i32
            * 0xe35adi32
            + (f64::from_le_bytes(concat_44(u_var9, u_var6).to_le_bytes())
                - (d_var1 + d_var1) * d_var11) as i32
                * -0x2309fb
            + (f64::from_le_bytes(concat_44(u_var7, u_var4).to_le_bytes()) - d_var2) as i32
                * 0x28842;
    }
}
