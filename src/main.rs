mod app;
mod cli;
mod config;

fn main() {
    let args = cli::Args::parse();
    if args.opt_paths.is_empty() {
        return;
    }
    app::run(args, None).expect("app");
}
