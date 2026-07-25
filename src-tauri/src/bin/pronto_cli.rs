fn main() {
    pronto_lib::core::run_cli(std::env::args().skip(1).collect());
}
