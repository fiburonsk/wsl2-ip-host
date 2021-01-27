use std::{io::BufRead, str::FromStr};

#[cfg(target_family = "unix")]
mod util {
    pub const DEFAULT_HOSTS_PATH: &str = "/mnt/c/Windows/System32/drivers/etc/hosts";

    pub fn run_wsl_ip_cmd() -> Result<std::process::Output, String> {
        let args = vec!["-4", "-br", "address", "show", "eth0"];

        std::process::Command::new("ip")
            .args(args)
            .output()
            .map_err(|e| format!("{}", e))
    }
}

#[cfg(target_family = "windows")]
mod util {
    pub const DEFAULT_HOSTS_PATH: &str = "C:\\Windows\\System32\\drivers\\etc\\hosts";

    pub fn run_wsl_ip_cmd() -> Result<std::process::Output, String> {
        let args = vec!["--", "ip", "-4", "-br", "address", "show", "eth0"];

        std::process::Command::new("wsl.exe")
            .args(args)
            .output()
            .map_err(|e| format!("{}", e))
    }
}

pub const HOSTS_COMMENT: &str = "# added by wsl2-ip-host";
pub const DEFAULT_HOST: &str = "host.wsl.internal";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn find_wsl_ip() -> Result<String, String> {
    let output = util::run_wsl_ip_cmd()?;
    if false == output.status.success() {
        return Err(
            String::from_utf8(output.stderr).unwrap_or("Unable to run ip command.".to_owned())
        );
    }

    let txt = String::from_utf8(output.stdout).map_err(|e| format!("{}", e))?;

    let ip = match txt.split_whitespace().last() {
        Some(text) => Ok(text.to_owned()),
        None => Err("Unable to split output text.".to_owned()),
    }?;

    match ip.split("/").next() {
        Some(ip) => Ok(ip.to_owned()),
        None => Err("Unable to separate IP from subnet".to_owned()),
    }
}

pub fn clean_list(list: &[String]) -> Vec<String> {
    list.to_owned()
        .into_iter()
        .filter_map(|line| {
            if line.contains(HOSTS_COMMENT) {
                None
            } else {
                Some(line)
            }
        })
        .collect()
}

#[derive(Clone)]
pub struct Config {
    pub hosts_path: std::path::PathBuf,
    pub names: Vec<String>,
    pub ip: Option<String>,
}

fn check_hosts_path(path: &str) -> Result<std::path::PathBuf, String> {
    if let Ok(p) = std::path::PathBuf::from_str(path) {
        match p
            .metadata()
            .map_err(|_| "Unable to gather metadata.".to_owned())
            .and_then(|m| Ok(m.permissions().readonly()))
        {
            Ok(false) => Ok(p),
            Ok(true) => Err(format!("path is not writable: {:?}", path)),
            Err(e) => Err(e),
        }
    } else {
        Err(format!("Unable to create path from {}", path).to_owned())
    }
}

impl Config {
    pub fn new() -> Result<Config, String> {
        let c = Config::with_hosts_path(util::DEFAULT_HOSTS_PATH)?;

        Ok(c)
    }

    pub fn set_hosts_path(&mut self, path: &str) {
        self.hosts_path = match check_hosts_path(path) {
            Ok(p) => p,
            _ => return,
        };
    }

    pub fn set_names(&mut self, list: Vec<String>) {
        self.names = list;
    }

    pub fn add_name(&mut self, name: String) {
        if false == self.names.contains(&name) {
            self.names.push(name);
        }
    }

    pub fn remove_name(&mut self, name: String) {
        self.names = self
            .names
            .clone()
            .into_iter()
            .filter(|n| n.ne(&name))
            .collect();
    }

    pub fn with_hosts_path(path: &str) -> Result<Config, String> {
        let p = check_hosts_path(path)?;

        Ok(Config {
            hosts_path: p,
            names: vec![],
            ip: None,
        })
    }

    pub fn read_file(&self) -> Result<Vec<String>, String> {
        let file = std::fs::File::open(&self.hosts_path).map_err(|e| format!("{}", e))?;
        let reader = std::io::BufReader::new(file);

        Ok(reader.lines().filter_map(|s| s.ok()).collect())
    }

    pub fn apply_names(&self, lines: &[String]) -> Vec<String> {
        let mut list = lines.to_owned();

        if let Some(ip) = &self.ip {
            list.extend(
                self.names
                    .iter()
                    .map(|name| format!("{} {} {}", ip, name, HOSTS_COMMENT)),
            );
        }

        list
    }

    pub fn load_ip(&mut self) {
        self.ip = find_wsl_ip().ok();
    }

    pub fn write_file(&self) -> Result<(), String> {
        let lines = self.read_file()?;
        let lines = clean_list(&lines);
        let lines = self.apply_names(&lines);

        use std::io::Write;
        let mut file = std::fs::File::create(&self.hosts_path).map_err(|e| format!("{}", e))?;

        lines.iter().for_each(|l| {
            writeln!(&mut file, "{}", l).unwrap();
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
