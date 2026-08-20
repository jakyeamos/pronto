#[allow(unused_variables)]
fn run_cli_arm_37(
    arguments: &Vec<String>,
    json: bool,
    path: &PathBuf,
    command: &str,
) {
    eprintln!("Unknown command: {command}");
    print_cli_usage();
    std::process::exit(2);
}
