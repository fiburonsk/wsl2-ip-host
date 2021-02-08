#![windows_subsystem = "windows"]
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Unsupported OS");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
mod ui;

fn main() {
    if let Err(e) = app::run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
mod app {
    use crate::ui;
    use main as lib;
    use serde::{Deserialize, Serialize};
    use serde_json;
    use std::sync::mpsc;
    use std::sync::RwLock;

    pub enum Cmd {
        AddName(String),
        Content(String),
        None,
        OnInit,
        Preview,
        Quit,
        ReadFile,
        RemoveName(String),
        SaveConfig,
        SetHostsFile(String),
        State(lib::Config),
        Write,
    }
    #[derive(Serialize, Deserialize)]
    struct SaveConfig {
        hosts_path: String,
        domains: Vec<String>,
    }

    const SAVE_NAME: &str = ".wsl2-ip-host.json";

    fn save_config(config: &lib::Config) -> Result<(), String> {
        let save = {
            SaveConfig {
                hosts_path: config.hosts_path.to_owned(),
                domains: config.names.to_owned(),
            }
        };

        let json = serde_json::to_string_pretty(&save).map_err(|e| format!("{}", e))?;
        let path = save_path()?;

        std::fs::write(path, json).map_err(|e| format!("{}", e))
    }

    fn save_path() -> Result<std::path::PathBuf, String> {
        match home::home_dir() {
            Some(p) => Ok(p.join(SAVE_NAME)),
            None => return Err("Unable to locate home folder.".to_owned()),
        }
    }

    fn read_config() -> Result<lib::Config, String> {
        let path = save_path()?;
        let config = if let Ok(content) = std::fs::read(path) {
            let state: SaveConfig =
                serde_json::from_slice(&content).map_err(|e| format!("{}", e))?;
            let mut config = lib::Config::with_hosts_path(&state.hosts_path);
            config.names = state.domains.to_owned();

            config
        } else {
            let mut config = lib::Config::new();
            config.add_name(lib::DEFAULT_HOST.to_owned());

            config
        };

        Ok(config)
    }

    fn notify(ip: &str, domains: &str) {
        let text = domains
            .split(",")
            .map(|name| format!("{} {}", &ip, name))
            .collect::<Vec<String>>()
            .join("\r\n");

        match notify_rust::Notification::new()
            .timeout(5000)
            .summary("Wrote to hosts file")
            .body(&format!(
                "Applied the following domains to the hosts file. \r\n\r\n{}",
                &text
            ))
            .show()
        {
            Ok(_) => (),
            Err(_) => (),
        }
    }

    pub fn run() -> Result<(), String> {
        let state = read_config()?;

        let state = RwLock::new(state);
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (main_tx, main_rx) = mpsc::channel();

        let handle = std::thread::spawn(|| {
            ui::begin(cmd_tx, main_rx);
        });

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                Cmd::OnInit => {
                    match state.read() {
                        Ok(s) => {
                            main_tx.send(Cmd::State(s.clone())).unwrap();

                            if std::env::args().any(|a| a == "--run") {
                                match write_changes(&s) {
                                    Ok(()) => (),
                                    _ => (),
                                };
                            }
                        }
                        _ => main_tx
                            .send(Cmd::Content("Unable to initialize state.".to_owned()))
                            .unwrap(),
                    };
                }
                Cmd::AddName(name) => {
                    if let Ok(mut s) = state.write() {
                        s.add_name(name);
                    }

                    match state.read() {
                        Ok(s) => main_tx.send(Cmd::State(s.clone())).unwrap(),
                        _ => main_tx.send(Cmd::None).unwrap(),
                    };
                }

                Cmd::RemoveName(name) => {
                    if let Ok(mut s) = state.write() {
                        s.remove_name(name);
                    }

                    match state.read() {
                        Ok(s) => main_tx.send(Cmd::State(s.clone())).unwrap(),
                        _ => main_tx.send(Cmd::None).unwrap(),
                    };
                }

                Cmd::SetHostsFile(path) => {
                    match state.write() {
                        Ok(mut s) => {
                            s.set_hosts_path(&path);
                            main_tx.send(Cmd::State(s.clone())).unwrap();
                        }
                        _ => main_tx.send(Cmd::None).unwrap(),
                    };
                }

                Cmd::ReadFile => {
                    match state.read() {
                        Ok(s) => match s.read_file() {
                            Ok(c) => main_tx
                                .send(Cmd::Content(c.join("\r\n").to_owned()))
                                .unwrap(),
                            _ => main_tx.send(Cmd::None).unwrap(),
                        },
                        _ => main_tx.send(Cmd::None).unwrap(),
                    };
                }

                Cmd::Preview => match state.write() {
                    Ok(mut s) => match s
                        .read_file()
                        .and_then(|l| Ok(lib::clean_list(&l)))
                        .and_then(|l| Ok(s.apply_names(&l)))
                    {
                        Ok(l) => main_tx.send(Cmd::Content(l.join("\r\n"))).unwrap(),
                        Err(_) => main_tx.send(Cmd::None).unwrap(),
                    },
                    _ => main_tx.send(Cmd::None).unwrap(),
                },

                Cmd::SaveConfig => {
                    match state.read() {
                        Ok(s) => match save_config(&s) {
                            Ok(()) => main_tx
                                .send(Cmd::Content(format!(
                                    "saved to {}",
                                    save_path().unwrap().to_str().unwrap().to_owned()
                                )))
                                .unwrap(),
                            Err(e) => main_tx.send(Cmd::Content(format!("{}", e))).unwrap(),
                        },
                        _ => main_tx
                            .send(Cmd::Content("Unable to read app state.".to_owned()))
                            .unwrap(),
                    };
                }

                Cmd::Write => {
                    match state.read() {
                        Ok(s) => match write_changes(&s) {
                            Ok(()) => main_tx.send(Cmd::Content("Saved.".to_owned())).unwrap(),
                            Err(e) => main_tx.send(Cmd::Content(e.to_owned())).unwrap(),
                        },
                        _ => main_tx
                            .send(Cmd::Content("Unable to read app state.".to_owned()))
                            .unwrap(),
                    };
                }

                Cmd::Quit => {
                    break;
                }
                _ => {}
            }
        }

        drop(cmd_rx);

        handle.join().unwrap();
        Ok(())
    }

    fn write_changes(state: &lib::Config) -> Result<(), String> {
        use std::ffi::OsStr;
        use std::iter::once;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;
        use winapi::ctypes::c_int;
        use winapi::um::shellapi::ShellExecuteW;

        let verb: Vec<u16> = OsStr::new("open").encode_wide().chain(once(0)).collect();

        let ip = lib::find_wsl_ip()?;
        let names = state.names.join(",");
        let path = &state.hosts_path;

        let file: Vec<u16> = OsStr::new("wsl2-ip-host-writer.exe")
            .encode_wide()
            .chain(once(0))
            .collect();

        let args: Vec<u16> = OsStr::new(format!("{} {} {}", ip, names, path).as_str())
            .encode_wide()
            .chain(once(0))
            .collect();

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
            notify(&ip, &names);
            Ok(())
        } else {
            Err("Unable to run wsl2-ip-host-writer.".to_owned())
        }
    }
}
