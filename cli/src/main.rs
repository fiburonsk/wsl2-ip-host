#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Unsupported OS");
    std::process::exit(1);
}

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
mod cli {
    use lib::find_wsl_ip;
    use main as lib;

    fn show_help() {
        print!(
            "wsl2-ip-host {}

Usage: wsl2-ip-host [-d distro] [-n <host-name>] ...

Uses wsl to retrieve the IP address of a wsl vm and writes it to the windows hosts
file.

Options:
-d, --distro <distro>       WSL distro name -d passed to wsl.exe. Falls back to your
                            default distro if omitted.
-n, --name <host-name>      Host name to associate the ip to [default: {}]
                            this option can be passed multiple times to add more than one
                            host name.
-h, --help                  Display help text
",
            lib::VERSION,
            lib::DEFAULT_HOST
        );
    }

    #[derive(Debug)]
    struct App {
        help: bool,
        names: Vec<String>,
        distro: Option<String>,
    }

    impl App {
        fn apply(&mut self, option: &str, value: Option<String>) {
            match option {
                "-d" | "--distro" if value.is_some() => self.distro = value,
                "-n" | "--name" if value.is_some() => self.names.push(value.unwrap()),
                "-n" | "--name" => (),
                _ => (),
            };
        }
    }

    fn parse_args() -> App {
        let args: Vec<String> = std::env::args().skip(1).collect();

        let mut cli = App {
            help: true,
            names: vec![],
            distro: None,
        };

        if args.iter().any(|a| &"-h" == a || &"--help" == a) {
            return cli;
        } else {
            cli.help = false;
        }

        let options = ["-d", "--distro", "-n", "--name"];
        let mut iter = args.into_iter().peekable();

        while let Some(text) = iter.next() {
            if options.contains(&&text[..]) {
                match iter.peek() {
                    Some(value) if !options.contains(&&value[..]) => {
                        cli.apply(&text, Some(value.to_owned()));
                        iter.next();
                    }
                    Some(_) => (),
                    None => cli.apply(&text, None),
                };
            }
        }

        if cli.names.is_empty() {
            cli.names.push(lib::DEFAULT_HOST.to_owned());
        }

        cli
    }

    pub fn run() -> Result<(), String> {
        let app = parse_args();

        if app.help {
            show_help();
            return Ok(());
        }

        let mut cfg = lib::Config::new();
        cfg.set_names(app.names.clone());
        cfg.distro = app.distro.clone();
        let ip = find_wsl_ip(&cfg.distro)?;
        lib::write_changes(&ip, &cfg)
    }
}
