mod app;
mod cli;
mod config;

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    if args.files.is_empty() {
        println!("{}", cli::HELP);
        return Ok(());
    }
    app::run(args, config::Config::default())
}

