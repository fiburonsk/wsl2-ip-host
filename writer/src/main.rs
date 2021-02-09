#![windows_subsystem = "windows"]
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Unsupported OS");
    std::process::exit(1);
}

fn main() {
    if let Err(e) = app::run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
mod app {
    use main as lib;

    pub fn run() -> Result<(), String> {
        let args = std::env::args();

        if args.len() != 4 {
            return Err("Insufficient arguments provided.".to_owned());
        }

        let mut i = args.skip(1);
        let ip = i.next().unwrap();
        let domains = i.next().unwrap();
        let path = i.next().unwrap();

        save(&path, &domains, &ip)
    }

    fn save(path: &str, domains: &str, ip: &str) -> Result<(), String> {
        let mut config = lib::Config::with_hosts_path(path);
        for d in domains.split(",") {
            config.add_name(d.to_owned());
        }

        config.write_file(ip)
    }
}
