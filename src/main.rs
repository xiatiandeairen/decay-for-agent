fn main() {
    let code = decay::cli::run().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        let mut src = std::error::Error::source(&e);
        while let Some(s) = src {
            eprintln!("  caused by: {s}");
            src = std::error::Error::source(s);
        }
        1
    });
    std::process::exit(code);
}
