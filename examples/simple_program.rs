pub fn main() {
    let i: u32 = 31337;

    let i_pointer = &i;
    println!("{:p}", i_pointer);
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
            _ => {}
        }

        input.clear();
    }
}
