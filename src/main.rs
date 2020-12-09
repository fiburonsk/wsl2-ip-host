const HOSTS_COMMENT: &str = "# added by wsl2-ip-host";
const DEFAULT_HOST: &str = "host.wsl.internal";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct Cli {
    help: bool,
    distro: Option<String>,
    name: Vec<String>,
}

impl Cli {
    fn apply(&mut self, option: &str, value: Option<String>) {
        match option {
            "-d" | "--distro" => self.distro = value.clone(),
            "-n" | "--name" if value.is_some() => self.name.push(value.unwrap()),
            "-n" | "--name" => (),
            _ => (),
        };
    }
}

#[cfg(target_family = "windows")]
fn find_wsl_ip(distro: &Option<String>) -> Result<String, String> {
    let mut args = vec![];

    if distro.is_some() {
        args.push("-d".to_owned());
        let name = distro.clone().unwrap();
        args.push(name);
    }

    args.push("--".to_owned());
    args.push("ip".to_owned());
    args.push("-4".to_owned());
    args.push("-br".to_owned());
    args.push("address".to_owned());
    args.push("show".to_owned());
    args.push("eth0".to_owned());

    std::process::Command::new("wsl.exe")
        .args(args)
        .output()
        .map_err(|e| format!("{}", e))
        .and_then(|output: std::process::Output| {
            if false == output.status.success() {
                return Err(String::from_utf8(output.stderr)
                    .unwrap_or("Unable to run ip command.".to_owned()));
            }

            String::from_utf8(output.stdout)
                .map_err(|e| format!("{}", e))
                .and_then(|s| match s.split_whitespace().last() {
                    Some(text) => Ok(text.to_owned()),
                    None => Err("Unable to split output text.".to_owned()),
                })
                .and_then(|ip| match ip.split("/").next() {
                    Some(ip) => Ok(ip.to_owned()),
                    None => Err("Unable to separate IP from subnet".to_owned()),
                })
        })
}

#[cfg(target_family = "unix")]
fn find_wsl_ip() -> Result<String, String> {
    let args = vec!["-4", "-br", "address", "show", "eth0"];

    std::process::Command::new("ip")
        .args(args)
        .output()
        .map_err(|e| format!("{}", e))
        .and_then(|output: std::process::Output| {
            if false == output.status.success() {
                return Err(String::from_utf8(output.stderr)
                    .unwrap_or("Unable to run ip command.".to_owned()));
            }

            String::from_utf8(output.stdout)
                .map_err(|e| format!("{}", e))
                .and_then(|s| match s.split_whitespace().last() {
                    Some(text) => Ok(text.to_owned()),
                    None => Err("Unable to split output text.".to_owned()),
                })
                .and_then(|ip| match ip.split("/").next() {
                    Some(ip) => Ok(ip.to_owned()),
                    None => Err("Unable to separate IP from subnet".to_owned()),
                })
        })
}

fn parse_args() -> Cli {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut cli = Cli {
        help: true,
        distro: None,
        name: vec![],
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

    if cli.name.is_empty() {
        cli.name.push(DEFAULT_HOST.to_owned());
    }

    cli
}

fn show_help() {
    print!(
        "wsl2-ip-host {}

Usage: wsl2-ip-host [-d <distro-name>] [-n <host-name>] ...

Uses wsl to retrieve the IP address of a wsl vm and writes it to the windows hosts
file. Testing so far seems to indicate that all wsl2 distros return the same IP
address so the -d option may not be important.

Options:
-d, --distro <distro-name>  Distro name to use.  This is passed into the `wsl`
                            command. When empty the default wsl distro is used.
-n, --name <host-name>      Host name to associate the ip to [default: {}]
                            this option can be passed multiple times to add more than one
                            host name.
-h, --help                  Display help text
",
        VERSION, DEFAULT_HOST
    );
}

fn run_app() -> Result<(), String> {
    let hosts_path = if cfg!(unix) {
        "/mnt/c/Windows/System32/drivers/etc/hosts"
    } else if cfg!(windows) {
        "C:\\Windows\\System32\\drivers\\etc\\hosts"
    } else {
        return Err("Unable to determine hosts file path from the OS".into());
    };

    let cli = parse_args();

    if cli.help {
        show_help();
        return Ok(());
    }

    let hosts_path = std::path::PathBuf::from(hosts_path);

    let file = match std::fs::File::open(&hosts_path) {
        Ok(f) => f,
        Err(e) => return Err(format!("{}", e)),
    };

    #[cfg(target_family = "windows")]
    let ip = match find_wsl_ip(&cli.distro) {
        Ok(s) => s,
        Err(e) => return Err(e),
    };

    #[cfg(target_family = "unix")]
    let ip = match find_wsl_ip() {
        Ok(s) => s,
        Err(e) => return Err(e),
    };

    let reader = std::io::BufReader::new(file);

    use std::io::BufRead;
    let mut lines: Vec<String> = reader
        .lines()
        .filter_map(|line_result| {
            let line: String = line_result.unwrap();
            if line.contains(HOSTS_COMMENT) {
                None
            } else {
                Some(line)
            }
        })
        .collect();

    cli.name.iter().for_each(|name| {
        lines.push(format!("{} {} {}", ip, name, HOSTS_COMMENT));
    });

    use std::io::Write;
    let mut file = match std::fs::File::create(&hosts_path) {
        Ok(f) => f,
        Err(e) => return Err(format!("{}", e)),
    };

    lines.iter().for_each(|l| {
        writeln!(&mut file, "{}", l).unwrap();
    });

    Ok(())
}

fn main() {
    if let Err(e) = run_app() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
