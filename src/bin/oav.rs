fn main() {
    let (format, result) = oav::run();
    if let Err(err) = result {
        if format == oav::OutputFormat::Json {
            let json_err = serde_json::json!({
                "error": format!("{err:#}"),
                "exit_code": oav::EXIT_INFRA_ERROR,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&json_err).expect("failed to serialize error")
            );
        } else {
            eprintln!("{err:#}");
        }
        std::process::exit(oav::EXIT_INFRA_ERROR);
    }
}
