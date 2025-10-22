static READONLY_VALUE: u32 = 12345;

pub fn main() {
    let i: u32 = 31337;

    let i_pointer = &i;
    println!("{:p}", i_pointer);

    let readonly_pointer = &READONLY_VALUE;
    println!("{:p}", readonly_pointer);

    let mut input = String::new();

    loop {
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let cmd = input.trim().to_lowercase();
        let cmd_str = cmd.as_str();

        match cmd_str {
            "read" => println!("{i}"),
            "addr" => println!("{:p}", i_pointer),
            "readonly" => println!("{}", READONLY_VALUE),
            "readonly_addr" => println!("{:p}", readonly_pointer),
            _ => {}
        }

        input.clear();
    }
}
