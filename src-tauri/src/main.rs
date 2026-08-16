// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|argument| argument == "--skill-usage-collector") {
        if let Err(error) = pronto_lib::skill_usage_collector::run() {
            eprintln!("Pronto skill usage collector failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    pronto_lib::run();
}
