mod core;
mod tui;

fn main() {
    if let Err(e) = tui::run() {
        panic!("{}", e);
    }
}
