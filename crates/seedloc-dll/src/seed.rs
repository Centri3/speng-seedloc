// Ghidra's Internal Decompiler Functions

fn concat_44(x: u32, y: u32) -> u64 {
    let mut bytes = vec![];

    for byte in &y.to_le_bytes() {
        bytes.push(*byte);
    }

    for byte in &x.to_le_bytes() {
        bytes.push(*byte);
    }

    u64::from_le_bytes(bytes.try_into().unwrap())
}

// Decompiled by Ghidra, thanks!
pub fn hash(point: u128) -> f64 {
    let upper = (point >> 64u32) as u64;
    let lower = point as u64;

    let mut f = upper;
    let mut s = lower;

    if upper > i64::MAX as u64 {
        f = !upper;
        s = !lower + 1u64;
        if lower == 0u64 {
            f += 1u64;
        }
    }

    let hashed = (s & u32::MAX as u64) as f64 * 8.271806125530277e-25f64
        + (f & u32::MAX as u64) as f64 * 1.52587890625e-05f64
        + (s >> 32u64) as f64 * 3.552713678800501e-15f64
        + (f >> 32u64) as f64 * 65536.0f64;

    if upper > i64::MAX as u64 {
        return -hashed;
    }

    hashed
}

// Also decompiled by Ghidra, thanks again!
pub fn seed(coords: (u128, u128, u128)) -> i32 {
    let mut d_var1: f64;

    let d_var10 = 100.0;
    let d_var11 = 2147483647.0f64;
    let d_var12 = 0.5f64;

    d_var1 = hash(coords.0) * d_var10;
    let u_var4 = (u64::from_le_bytes(d_var1.to_le_bytes())) as u32;
    let u_var7 = (u64::from_le_bytes(d_var1.to_le_bytes()) >> 0x20u64) as u32;
    d_var1 = f64::floor(d_var1 / d_var11 * d_var12 + d_var12);
    let d_var2 = (d_var1 + d_var1) * d_var11;

    d_var1 = hash(coords.1) * d_var10;
    let u_var5 = (u64::from_le_bytes(d_var1.to_le_bytes())) as u32;
    let u_var8 = (u64::from_le_bytes(d_var1.to_le_bytes()) >> 0x20u64) as u32;
    d_var1 = f64::floor(d_var1 / d_var11 * d_var12 + d_var12);
    let d_var3 = (d_var1 + d_var1) * d_var11;

    d_var1 = hash(coords.2) * d_var10;
    let u_var6 = (u64::from_le_bytes(d_var1.to_le_bytes())) as u32;
    let u_var9 = (u64::from_le_bytes(d_var1.to_le_bytes()) >> 0x20u64) as u32;
    d_var1 = f64::floor(d_var1 / d_var11 * d_var12 + d_var12);

    (f64::from_le_bytes(concat_44(u_var8, u_var5).to_le_bytes()) - d_var3) as i32 * 0xe35adi32
        + (f64::from_le_bytes(concat_44(u_var9, u_var6).to_le_bytes())
            - (d_var1 + d_var1) * d_var11) as i32
            * -0x2309fb
        + (f64::from_le_bytes(concat_44(u_var7, u_var4).to_le_bytes()) - d_var2) as i32 * 0x28842
}
