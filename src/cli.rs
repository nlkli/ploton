use std::path::PathBuf;

const VERSION: &str = concat!(
    "ploton ",
    env!("CARGO_PKG_VERSION"),
    " [https://github.com/nlkli/ploton]"
);

pub const HELP: &str = "\
ploton - render ECharts options and CSV datasets in a native window

Usage: ploton [OPTIONS] <FILE>...

Arguments:
  <FILE>         ECharts option (JS object literal or JSON) or a .csv dataset

Options:
  -w, --watch    Reload the next <FILE> whenever it changes
  -h, --help     Print help
  -V, --version  Print version

Examples:
  ploton option.js
  ploton -w data.csv";

/// An input file and whether it should be reloaded on change.
#[derive(Clone, Debug)]
pub struct InputFile {
    pub path: PathBuf,
    pub watch: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Args {
    pub files: Vec<InputFile>,
}

impl Args {
    pub fn parse() -> Self {
        let mut files = Vec::new();
        // `--watch` applies only to the file that follows it.
        let mut watch_next = false;

        for arg in std::env::args().skip(1) {
            if let Some(long) = arg.strip_prefix("--") {
                match long {
                    "watch" => watch_next = true,
                    "help" => exit_with(HELP),
                    "version" => exit_with(VERSION),
                    _ => usage_error(&arg),
                }
            } else if arg.len() > 1 && arg.starts_with('-') {
                for flag in arg[1..].chars() {
                    match flag {
                        'w' => watch_next = true,
                        'h' => exit_with(HELP),
                        'V' => exit_with(VERSION),
                        _ => usage_error(&format!("-{flag}")),
                    }
                }
            } else {
                let path = PathBuf::from(&arg);
                if !path.is_file() {
                    eprintln!("Error: file not found: '{arg}'");
                    std::process::exit(1);
                }
                files.push(InputFile {
                    path,
                    watch: std::mem::take(&mut watch_next),
                });
            }
        }

        Self { files }
    }
}

fn exit_with(text: &str) -> ! {
    println!("{text}");
    std::process::exit(0);
}

fn usage_error(option: &str) -> ! {
    eprintln!("Error: unknown option '{option}'\n\n{HELP}");
    std::process::exit(2);
}
