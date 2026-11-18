fn main() {
    let args: Vec<String> = std::env::args().collect();
    ferrum_doom::i_main::run_doom(args);
}
