fn main() {
    if let Err(e) = cli::run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

mod cli {
    use main as lib;

    fn show_help() {
        print!(
            "wsl2-ip-host {}

Usage: wsl2-ip-host [-d <distro-name>] [-n <host-name>] ...

Uses wsl to retrieve the IP address of a wsl vm and writes it to the windows hosts
file. Testing so far seems to indicate that all wsl2 distros return the same IP
address so the -d option may not be important.

Options:
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
    }

    impl App {
        fn apply(&mut self, option: &str, value: Option<String>) {
            match option {
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
        };

        if args.iter().any(|a| &"-h" == a || &"--help" == a) {
            return cli;
        } else {
            cli.help = false;
        }

        let options = ["-n", "--name"];
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
        cfg.write_file()
    }
}
