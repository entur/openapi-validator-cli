fn main() {
    if let Err(err) = oav::run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}
