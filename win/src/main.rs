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
        InitOk,
        Distros(Vec<String>),
        SetDistro(String),
        Error(String),
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
        distro: Option<String>,
    }

    const SAVE_NAME: &str = ".wsl2-ip-host.json";

    fn save_config(config: &lib::Config) -> Result<(), String> {
        let save = {
            SaveConfig {
                hosts_path: config.hosts_path.to_owned(),
                domains: config.names.to_owned(),
                distro: config.distro.to_owned(),
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
            config.distro = state.distro;

            config
        } else {
            let mut config = lib::Config::new();
            config.add_name(lib::DEFAULT_HOST.to_owned());

            config
        };

        Ok(config)
    }

    fn notify(ip: &str, domains: &Vec<String>) {
        let text = domains
            .iter()
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
        let unable_to_read = "Unable to read app state.";
        let state = read_config().unwrap_or(lib::Config::new());
        let state = RwLock::new(state);
        let distros = lib::find_wsl_distros().unwrap_or(vec![]);
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (main_tx, main_rx) = mpsc::channel();

        let handle = std::thread::spawn(|| {
            ui::begin(cmd_tx, main_rx);
        });

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                Cmd::OnInit => match state.read() {
                    Ok(s) => {
                        main_tx.send(Cmd::InitOk).unwrap();
                        main_tx.send(Cmd::Distros(distros.to_owned())).unwrap();
                        main_tx.send(Cmd::State(s.clone())).unwrap();

                        if std::env::args().any(|a| a == "--run") {
                            match lib::find_wsl_ip(&s.distro) {
                                Ok(ip) => match lib::write_changes(&ip, &s) {
                                    Ok(_) => notify(&ip, &s.names),
                                    Err(_) => (),
                                },
                                _ => (),
                            };
                        }
                    }
                    _ => main_tx
                        .send(Cmd::Content("Unable to initialize state.".to_owned()))
                        .unwrap(),
                },
                Cmd::SetDistro(name) => match state.write() {
                    Ok(mut s) => {
                        s.distro = Some(name);
                    }
                    _ => (),
                },
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

                Cmd::SetHostsFile(path) => match state.write() {
                    Ok(mut s) => {
                        s.set_hosts_path(&path);
                        main_tx.send(Cmd::State(s.clone())).unwrap();
                    }
                    _ => main_tx.send(Cmd::Error(unable_to_read.to_owned())).unwrap(),
                },

                Cmd::ReadFile => match state.read() {
                    Ok(s) => match s.read_file() {
                        Ok(c) => main_tx
                            .send(Cmd::Content(c.join("\r\n").to_owned()))
                            .unwrap(),
                        Err(s) => main_tx.send(Cmd::Error(s)).unwrap(),
                    },
                    _ => main_tx.send(Cmd::Error(unable_to_read.to_owned())).unwrap(),
                },

                Cmd::Preview => match state.read() {
                    Ok(s) => match lib::find_wsl_ip(&s.distro) {
                        Ok(ip) => match s.preview(&ip) {
                            Ok(l) => main_tx.send(Cmd::Content(l.join("\r\n"))).unwrap(),
                            Err(s) => main_tx.send(Cmd::Error(s)).unwrap(),
                        },
                        Err(s) => main_tx.send(Cmd::Error(s)).unwrap(),
                    },
                    _ => main_tx.send(Cmd::Error(unable_to_read.to_owned())).unwrap(),
                },

                Cmd::SaveConfig => match state.read() {
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
                        .send(Cmd::Content(unable_to_read.to_owned()))
                        .unwrap(),
                },

                Cmd::Write => match state.read() {
                    Ok(s) => match lib::find_wsl_ip(&s.distro) {
                        Ok(ip) => match lib::write_changes(&ip, &s) {
                            Ok(()) => {
                                main_tx.send(Cmd::Content("Saved.".to_owned())).unwrap();
                                notify(&ip, &s.names);
                            }
                            Err(e) => main_tx.send(Cmd::Content(e.to_owned())).unwrap(),
                        },
                        Err(s) => main_tx.send(Cmd::Content(s)).unwrap(),
                    },
                    _ => main_tx
                        .send(Cmd::Content(unable_to_read.to_owned()))
                        .unwrap(),
                },

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
}
