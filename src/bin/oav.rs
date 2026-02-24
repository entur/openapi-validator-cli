fn main() {
    if let Err(err) = oav::run() {
        eprintln!("{err:#}");
        std::process::exit(oav::EXIT_INFRA_ERROR);
    }
}
