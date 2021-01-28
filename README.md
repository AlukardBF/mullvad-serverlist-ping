# mullvad-serverlist-ping

Get the list of the servers with best latency.

```Batchfile
USAGE:
    mullvad-serverlist-ping.exe [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --display <display>     Number of servers with best latency to display [default: 10]
    -n, --count <ping-count>    Number of echo requests to send [default: 4]
    -t, --type <vpn-type>       Type of vpn ('wireguard' or 'openvpn') [default: wireguard]
```
