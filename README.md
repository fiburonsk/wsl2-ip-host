# wsl2-ip-host

## From windows

* Windows 10
* Requires that wsl.exe be installed and in your path.
* Access to write to the windows hosts file
* A wsl distro be installed
* `ip` command available in a wsl distro with eth0 adapter

This application runs `ip -4 -br address show eth0` inside of the default wsl distro to get the IP address and then appends it to the hosts file with a domain of `host.wsl.internal`.  The domain can be changed using the `-n` or `--name` option.  You can supply multiple domains by passing the `--name` option multiple times.

I use a scheduled task in windows to run this on login and refresh the wsl ip in the hosts file since the IP changes after every reboot.

### Windows GUI

`wsl2-ip-host-gui.exe` is an alternative way to run the app.  It runs in the 
background with a system tray icon.  Right clicking and selecting `Write` will overwrite the hosts file.  A configuration file can be saved in your home folder as `.wsl2-ip-host.json` and will automatically attempt to be read on startup.  If using in a scheduled task you can pass the `--run` argument to trigger a write immediately after initialization: `wsl2-ip-host-gui.exe --run`.  Selecting open allows configuring the app and saving for future runs.  The hosts file can be viewed and the changes previewed.

## From WSL2

* Write access to `/mnt/c/Windows/System32/Drivers/etc/hosts`
* `ip` command and an eth0 adapter

This command could be placed inside a .bashrc or equivalent but will only run when a new shell is started rather than automatically when wsl is started.
