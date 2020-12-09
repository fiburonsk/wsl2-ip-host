# wsl2-ip-host

##2 From windows

* Windows 10
* Requires that wsl.exe be installed and in your path.
* Access to write to the windows hosts file
* A wsl distro be installed
* `ip` command available in a wsl distro with eth0 adapter

This application runs `ip -4 -br address show eth0` inside of a wsl distro to get the IP address and then appends it to the hosts file with a domain of `host.wsl.internal`.  The domain can be changed using the `-n` or `--name` option.  You can supply multiple domains by passing the `--name` option multiple times.

I use a scheduled task in windows to run this on login and refresh the wsl ip in the hosts file since the IP changes after every reboot.

## From WSL2

* Write access to `/mnt/c/Windows/System32/Drivers/etc/hosts`
* `ip` command and an eth0 adapter

This command could be placed inside a .bashrc or equivalent but will only run when a new shell is started rather than automatically when wsl is started.
