use faccess::PathExt;
use std::io::BufRead;
use util::WRITER_EXE;

mod util {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    pub const HOSTS_COMMENT: &str = "# added by wsl2-ip-host";
    pub const WRITER_EXE: &str = "wsl2-ip-host-writer.exe";

    pub fn run_wsl_ip_cmd(distro: &Option<String>) -> Result<std::process::Output, String> {
        use std::os::windows::process::CommandExt;

        let mut args = if let Some(s) = distro {
            vec!["-d", &s[..]]
        } else {
            vec![]
        };

        args.append(&mut vec![
            "--", "ip", "-4", "-br", "address", "show", "eth0",
        ]);

        let mut cmd = std::process::Command::new("wsl.exe");

        cmd.args(args);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.output().map_err(|e| format!("{}", e))
    }

    pub fn run_wsl_list_distros() -> Result<std::process::Output, String> {
        use std::os::windows::process::CommandExt;

        let args = vec!["-l", "--all"];
        let mut cmd = std::process::Command::new("wsl.exe");

        cmd.args(args);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.output().map_err(|e| format!("{}", e))
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

    pub fn null_text(text: &str) -> Vec<u16> {
        use std::ffi::OsStr;
        use std::iter::once;
        use std::os::windows::ffi::OsStrExt;

        OsStr::new(text).encode_wide().chain(once(0)).collect()
    }
}

pub const DEFAULT_HOSTS_PATH: &str = "C:\\Windows\\System32\\drivers\\etc\\hosts";
pub const DEFAULT_HOST: &str = "host.wsl.internal";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// shells to wsl2-ip-host-writer to build a config and write
pub fn write_changes(ip: &str, state: &Config) -> Result<(), String> {
    use std::ptr;
    use winapi::ctypes::c_int;
    use winapi::um::shellapi::ShellExecuteW;

    let verb: Vec<u16> = util::null_text("open");
    let names = state.names.join(",");
    let path = &state.hosts_path;
    let file = util::null_text(WRITER_EXE);
    let args = util::null_text(format!("{} {} {}", ip, names, path).as_str());

    let ret = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            args.as_ptr(),
            ptr::null(),
            c_int::from(0),
        )
    };

    if ret as i32 > 31 {
        Ok(())
    } else {
        Err("Unable to run wsl2-ip-host-writer.".to_owned())
    }
}

pub fn find_writer() -> Option<String> {
    match std::process::Command::new(WRITER_EXE).spawn() {
        Err(s) if s.to_string().contains("requires elevation") => None,
        Err(s) if s.kind() == std::io::ErrorKind::NotFound => {
            Some(format!("wsl2-ip-host-writer.exe not found: {}", s))
        }
        Err(s) => Some(s.to_string()),
        Ok(_) => None,
    }
}

pub fn find_wsl_ip(distro: &Option<String>) -> Result<String, String> {
    let output = util::run_wsl_ip_cmd(distro)?;
    if false == output.status.success() {
        return Err("Unable to run ip command.".to_owned());
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

pub fn find_wsl_distros() -> Result<Vec<String>, String> {
    let output = util::run_wsl_list_distros()?;
    if false == output.status.success() {
        return Err(String::from_utf8(output.stderr)
            .unwrap_or("Unable to get a list of distros from wsl.exe.".to_owned()));
    }

    // wsl.exe outputs utf16 so convert the output to a [u16] from a [u8]
    let b: Vec<u16> = output
        .stdout
        .chunks_exact(2)
        .map(|c| u16::from_ne_bytes([c[0], c[1]]))
        .collect();

    let txt = String::from_utf16(&b).map_err(|e| format!("{}", e))?;

    Ok(txt.lines().skip(1).map(|l| l.trim().to_owned()).collect())
}

#[derive(Clone)]
pub struct Config {
    pub hosts_path: String,
    pub names: Vec<String>,
    pub distro: Option<String>,
}

pub struct Access {
    pub path: std::path::PathBuf,
    pub read: bool,
    pub write: bool,
}

impl Config {
    pub fn new() -> Config {
        Config::with_hosts_path(DEFAULT_HOSTS_PATH)
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
            distro: None,
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

    pub fn apply_names(&self, ip: &str, lines: &[String]) -> Vec<String> {
        let mut list = lines.to_owned();

        list.extend(
            self.names
                .iter()
                .map(|name| format!("{} {} {}", &ip, name, util::HOSTS_COMMENT)),
        );

        list
    }

    pub fn preview(&self, ip: &str) -> Result<Vec<String>, String> {
        let lines = self.read_file()?;
        let lines = util::clean_list(&lines);
        Ok(self.apply_names(ip, &lines))
    }

    pub fn write_file(&self, ip: &str) -> Result<(), String> {
        let access = self.check_hosts_path();

        if false == access.write {
            return Err(format!(
                "Insufficient access to write file {}",
                self.hosts_path
            ));
        }

        let lines = self.preview(ip)?;

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
