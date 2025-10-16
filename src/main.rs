mod core;
mod tui;

fn main() {
    match tui::run() {
        Err(e) => panic!("{}", e),
        Ok(_) => {}
    }
}
