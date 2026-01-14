fn main() {
    if let Err(err) = openapi_validator::run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}
