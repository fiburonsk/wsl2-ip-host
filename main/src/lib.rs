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

pub struct Config {
    pub hosts_path: std::path::PathBuf,
    pub names: Vec<String>,
}

impl Config {
    pub fn new() -> Result<Config, String> {
        let c = Config::with_hosts_path(util::DEFAULT_HOSTS_PATH)?;

        Ok(c)
    }

    pub fn set_names(&mut self, list: Vec<String>) {
        self.names = list;
    }

    pub fn with_hosts_path(path: &str) -> Result<Config, String> {
        let p =
            std::path::PathBuf::from_str(path).map_err(|_| "Unable to create path.".to_owned())?;

        match p
            .metadata()
            .map_err(|_| "Unable to gather metadata.".to_owned())
            .and_then(|m| Ok(m.permissions().readonly()))
        {
            Ok(false) => Ok(Config {
                hosts_path: p,
                names: vec![],
            }),
            Ok(true) => Err(format!("host path is not writable: {}", path)),
            Err(e) => Err(e),
        }
    }

    pub fn read_file(&self) -> Result<Vec<String>, String> {
        let file = std::fs::File::open(&self.hosts_path).map_err(|e| format!("{}", e))?;
        let reader = std::io::BufReader::new(file);

        Ok(reader.lines().filter_map(|s| s.ok()).collect())
    }

    pub fn apply_names(&self, ip: &str, lines: &[String]) -> Vec<String> {
        let mut list = lines.to_owned();
        list.extend(
            self.names
                .iter()
                .map(|name| format!("{} {} {}", ip, name, HOSTS_COMMENT)),
        );

        list
    }

    pub fn write_file(&self, lines: &Vec<String>) -> Result<(), String> {
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
