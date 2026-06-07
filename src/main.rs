use std::process;

fn main() {
    if let Err(err) = mdseek::cli::run(std::env::args()) {
        eprintln!("error: {err}");
        process::exit(1);
    }
}
