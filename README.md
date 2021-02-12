# wsl2-ip-host

* Windows 10
* Requires that wsl.exe be installed and in your path.
* A wsl distro be installed
* `ip` command available in a wsl distro with eth0 adapter

This application runs `ip -4 -br address show eth0` inside of the default wsl distro to get the IP address.  This ip address is used for writing entries into the hosts file.  The default domain is `host.wsl.internal`.  

I use wsl2-ip-host.exe as a scheduled task that begins on logon to write the wsl2 ip right away since it changes on restart.

`wsl2-ip-host.exe` and `wsl2-ip-host-cli.exe` both require that `wsl2-ip-host-writer.exe` be either in the same folder or in your path so that it can be found and run. 

## wsl2-ip-host.exe

A windows tool that lives in the system tray where a convenient write action is available.  Domains can be configured within the `open` window as well as selecting a different host file path in case it were to be needed.  The configuration can be saved through the menu option at the top.  The configuration is saved at ~/.wsl2-ip-host.json and this file is automatically loaded on startup.

## wsl2-ip-host-writer.exe

This application does the work of writing the IP to the hosts file.  It requires elevated privileges to run and will prompt first.

## wsl2-ip-host-cli.exe

A cli utility to call the writer and write changes to the hosts file.

```
Usage: wsl2-ip-host [-d distro] [-n <host-name>] ...

Uses wsl to retrieve the IP address of a wsl vm and writes it to the windows hosts  
file.

Options:
-d, --distro <distro>       WSL distro name -d passed to wsl.exe. Falls back to your
                            default distro if omitted.
-n, --name <host-name>      Host name to associate the ip to [default: host.wsl.internal]
                            this option can be passed multiple times to add more than one
                            host name.
-h, --help                  Display help text
```

The domain can be changed using the `-n` or `--name` option.  You can supply multiple domains by passing the `-n` or `--name` option multiple times. If the default WSL distro does not work you can use `-d` or `--distro` to provide a different distro to run the command against.

## Build

clone the repository and use `cargo build` or `cargo build --release`. I have only built this with the `stable-x86_64-pc-windows-msvc` toolchain.
