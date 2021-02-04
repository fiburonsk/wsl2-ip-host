use std::io::BufRead;

use faccess::PathExt;

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
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    pub const DEFAULT_HOSTS_PATH: &str = "C:\\Windows\\System32\\drivers\\etc\\hosts";

    pub fn run_wsl_ip_cmd() -> Result<std::process::Output, String> {
        let args = vec!["--", "ip", "-4", "-br", "address", "show", "eth0"];

        let mut cmd = std::process::Command::new("wsl.exe");
        cmd.args(args);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.output().map_err(|e| format!("{}", e))
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
    pub hosts_path: String,
    pub names: Vec<String>,
    pub ip: Option<String>,
}

pub struct Access {
    pub path: std::path::PathBuf,
    pub read: bool,
    pub write: bool,
}

impl Config {
    pub fn new() -> Config {
        Config::with_hosts_path(util::DEFAULT_HOSTS_PATH)
    }

    pub fn set_hosts_path(&mut self, path: &str) {
        self.hosts_path = path.to_owned();
    }

    pub fn check_hosts_path(&self) -> Access {
        let path = std::path::PathBuf::from(&self.hosts_path);

        Access {
            read: path.readable(),
            write: path.writable(),
            path: path,
        }
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

    pub fn with_hosts_path(path: &str) -> Config {
        Config {
            hosts_path: path.to_owned(),
            names: vec![],
            ip: None,
        }
    }

    pub fn read_file(&self) -> Result<Vec<String>, String> {
        let access = self.check_hosts_path();

        if false == access.read {
            return Err(format!("Unable to read file {}", self.hosts_path));
        }

        let file = std::fs::File::open(access.path).map_err(|e| format!("{}", e))?;
        let reader = std::io::BufReader::new(file);

        Ok(reader.lines().filter_map(|s| s.ok()).collect())
    }

    pub fn apply_names(&mut self, lines: &[String]) -> Vec<String> {
        let mut list = lines.to_owned();

        if let Ok(ip) = find_wsl_ip() {
            list.extend(
                self.names
                    .iter()
                    .map(|name| format!("{} {} {}", &ip, name, HOSTS_COMMENT)),
            );

            self.ip = Some(ip);
        }

        list
    }

    pub fn write_file(&mut self) -> Result<(), String> {
        let access = self.check_hosts_path();

        if false == access.write {
            return Err(format!(
                "Insufficient access to write file {}",
                self.hosts_path
            ));
        }

        let lines = self.read_file()?;
        let lines = clean_list(&lines);
        let lines = self.apply_names(&lines);

        use std::io::Write;
        let mut file = std::fs::File::create(access.path).map_err(|e| format!("{}", e))?;

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
