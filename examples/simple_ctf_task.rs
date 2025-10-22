static STATIC_TEXT: &'static str = "TEST_STATIC_STRING";

pub fn main() {
    // Construct flag at runtime using obfuscated mathematical operations
    // Each byte is computed from encoded values to avoid appearing in binary strings
    // Format: (multiplier, multiplicand, addend, subtractor)
    let encoded: Vec<(u8, u8, u8, u8)> = vec![
        (7, 5, 35, 0),  // F = 70 = 7*5 + 35
        (19, 2, 38, 0), // L = 76 = 19*2 + 38
        (13, 3, 26, 0), // A = 65 = 13*3 + 26
        (8, 4, 39, 0),  // G = 71 = 8*4 + 39
        (31, 2, 61, 0), // { = 123 = 31*2 + 61
        (14, 3, 28, 0), // F = 70 = 14*3 + 28
        (13, 2, 26, 0), // 4 = 52 = 13*2 + 26
        (15, 3, 30, 0), // K = 75 = 15*3 + 30
        (17, 2, 17, 0), // 3 = 51 = 17*2 + 17
        (19, 3, 38, 0), // _ = 95 = 19*3 + 38
        (35, 1, 35, 0), // F = 70 = 35*1 + 35
        (38, 1, 38, 0), // L = 76 = 38*1 + 38
        (26, 1, 26, 0), // 4 = 52 = 26*1 + 26
        (71, 1, 0, 0),  // G = 71 = 71*1 + 0
        (25, 3, 50, 0), // } = 125 = 25*3 + 50
    ];

    let xor_key = 137u8;
    let flag: String = encoded
        .iter()
        .enumerate()
        .map(|(i, &(mul1, mul2, add, sub))| {
            let mut val = mul1.wrapping_mul(mul2);
            val = val.wrapping_add(add);
            val = val.wrapping_sub(sub);
            val = val ^ (xor_key.wrapping_shr((i % 8) as u32) & 0x0F);
            val ^ (xor_key.wrapping_shr((i % 8) as u32) & 0x0F)
        })
        .map(|b| b as char)
        .collect();

    let flag_ptr = &flag;
    println!("{:p}", flag_ptr);
    let mut input = String::new();

    loop {
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let cmd = input.trim().to_lowercase();
        let cmd_str = cmd.as_str();

        match cmd_str {
            "p" => println!("{flag}"),
            "a" => println!("{:p}", flag_ptr),
            "s" => println!("{:p}", STATIC_TEXT),
            _ => {}
        }

        input.clear();
    }
}
