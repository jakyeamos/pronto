#[allow(unused_variables)]
fn run_cli_arm_06(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    match papercuts::run_cli(&arguments) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    if json {
                        print_cli_json_error("papercuts", &error);
                    } else {
                        eprintln!("Pronto Papercuts error: {error}");
                    }
                    std::process::exit(1);
                }
            }
}
