fn main() {
    let code = decay::cli::run().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        1
    });
    std::process::exit(code);
}
