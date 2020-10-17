# wsl2-ip-host

Requirements to run:

* Windows 10
* Requires that wsl.exe be installed and in your path.
* Access to write to the windows hosts file
* A wsl distro be installed
* `hostname` command with `-I` switch available in a wsl distro


This application runs `hostname -I` inside of a wsl distro to get the IP address and then appends it to the hosts file with a domain of `host.wsl.internal`.  The domain can be changed using the `-n` or `--name` option.  You can supply multiple domains by passing the `--name` option multiple times.

I use a scheduled task in windows to run this on login and refresh the wsl ip in the hosts file since the IP changes after every reboot.
