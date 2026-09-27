use std::path::PathBuf;

const VERSION: &str = "ploton 0.1.0 [https://github.com/nlkli/ploton]";
const HELP: &str = r#"chart
Usage: ploton [OPTIONS] [FILE]
Options:
  -w, --watch [FILE]
  -h, --help; -V, --version"#;

#[derive(Clone, Debug, Default)]
pub struct Args {
    // option file path and watch flag
    pub opt_paths: Vec<(PathBuf, bool)>,
}

impl Args {
    pub fn parse() -> Self {
        let mut args = Self::default();

        let mut last: Option<char> = None;

        let mut iter = std::env::args().skip(1);

        while let Some(arg) = iter.next() {
            if let Some(flag) = arg.strip_prefix("--") {
                match flag {
                    "watch" => last = Some('w'),
                    "help" => {
                        println!("{HELP}");
                        std::process::exit(0);
                    }

                    "version" => {
                        println!("{VERSION}");
                        std::process::exit(0);
                    }

                    _ => (),
                }
            } else if let Some(flags) = arg.strip_prefix('-') {
                for c_flag in flags.chars() {
                    match c_flag {
                        'w' => last = Some(c_flag),
                        'h' => {
                            println!("{HELP}");
                            std::process::exit(0);
                        }
                        'V' => {
                            println!("{VERSION}");
                            std::process::exit(0);
                        }

                        _ => (),
                    }
                }
            } else {
                match last.take() {
                    Some('w') => args.add_opt_path(&arg, true),
                    None => args.add_opt_path(&arg, false),

                    _ => (),
                }
            }
        }

        args
    }

    fn add_opt_path(&mut self, path_arg: &str, watch: bool) {
        let path = PathBuf::from(&path_arg);
        if path.is_file() {
            self.opt_paths.push((path, watch));
        } else {
            eprintln!("Error: file not found: '{}'", path_arg);
            std::process::exit(1);
        }
    }
}
