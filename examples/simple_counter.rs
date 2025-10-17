pub fn main() {
    let mut counter: i32 = 3;

    println!("Counter address: {:p}, Value: {counter}", &counter);
    println!("Commands: i, d, p");

    let mut input = String::new();

    loop {
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let cmd = input.trim().to_lowercase();
        let cmd_str = cmd.as_str();

        match cmd_str {
            "i" => counter += 1,
            "d" => counter -= 1,
            _ => {}
        }
        println!("{}", counter);
        input.clear();
    }
}
