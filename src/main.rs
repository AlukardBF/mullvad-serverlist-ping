use mullvad_pinger::{AsyncPinger, Server};
use std::{fs::File, io::Write};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "mullvad-serverlist-ping",
    about = "Get the list of the servers with best latency."
)]
pub struct Config {
    #[structopt(
        short = "n",
        long = "count",
        default_value = "4",
        help = "Number of echo requests to send"
    )]
    ping_count: u32,
    #[structopt(
        short = "t",
        long = "type",
        default_value = "wireguard",
        help = "Type of vpn ('wireguard' or 'openvpn')"
    )]
    vpn_type: String,
    #[structopt(
        short = "d",
        long = "display",
        default_value = "10",
        help = "Number of servers with best latency to display"
    )]
    display: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args();
    match config.vpn_type.as_str() {
        "wireguard" | "openvpn" => (),
        _ => panic!("Wrong vpn type!"),
    };
    let mullvad = "https://api.mullvad.net/www/relays/all/";
    let resp: Vec<Server> = reqwest::get(mullvad).await?.json().await?;
    let servers: Vec<Server> = resp
        .iter()
        .filter(|s| s.type_field == config.vpn_type && s.active)
        .map(|s| s.to_owned())
        .collect();

    let mut pinger = AsyncPinger::new(servers, config.ping_count);
    if let Some(servers) = pinger.ping_servers().await?.sort_best().result() {
        servers
            .iter()
            .take(config.display as usize)
            .for_each(|s| println!("{:#?}", s));
    }
    let mut file = File::create("servers.txt")?;
    pinger
        .result()
        .iter()
        .for_each(|s| writeln!(file, "{:#?}", s).expect("Unable to write to file"));
    Ok(())
}

mod mullvad_pinger {
    use futures::{stream::FuturesUnordered, StreamExt};
    use serde_derive::Deserialize;
    use std::{fmt::Error, iter::Take, net::IpAddr};

    #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
    pub struct Server {
        pub hostname: String,
        pub country_code: String,
        pub country_name: String,
        pub city_code: String,
        pub city_name: String,
        pub active: bool,
        pub owned: bool,
        pub provider: String,
        pub ipv4_addr_in: String,
        pub ipv6_addr_in: Option<String>,
        #[serde(rename = "type")]
        pub type_field: String,
        pub status_messages: Vec<StatusMessage>,
        pub pubkey: Option<String>,
        pub multihop_port: Option<i64>,
        pub socks_name: Option<String>,
        pub ssh_fingerprint_sha256: Option<String>,
        pub ssh_fingerprint_md5: Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Deserialize)]
    pub struct StatusMessage {
        pub message: String,
        pub timestamp: String,
    }

    #[derive(Default, Debug, Clone, PartialEq)]
    pub struct ServerPing {
        pub hostname: String,
        pub ping: u32,
    }

    pub struct AsyncPinger {
        servers: Vec<Server>,
        ping_count: u32,
        ping_results: Option<Vec<ServerPing>>,
    }

    impl AsyncPinger {
        pub fn new(servers: Vec<Server>, ping_count: u32) -> Self {
            Self {
                servers,
                ping_count,
                ping_results: None,
            }
        }
        pub async fn ping_servers(&mut self) -> Result<&mut Self, Error> {
            self.ping_results = Some(
                self.servers
                    .iter()
                    .map(|s| Self::ping_server(self, s))
                    .collect::<FuturesUnordered<_>>()
                    .collect::<Vec<_>>()
                    .await,
            );
            Ok(self)
        }
        async fn ping_server(&self, server: &Server) -> ServerPing {
            let ip = server.ipv4_addr_in.parse::<IpAddr>().unwrap();
            let pinger = winping::AsyncPinger::new();
            let mut average = 0u32;
            for _ in 0..self.ping_count {
                let buf = winping::Buffer::new();
                let ping = pinger.send(ip, buf).await;
                average += ping.result.unwrap_or(99999);
            }
            ServerPing {
                hostname: server.hostname.clone(),
                ping: average / self.ping_count,
            }
        }
        pub fn sort_best(&mut self) -> &Self {
            if let Some(s) = self.ping_results.as_deref_mut() {
                s.sort_by(|a, b| a.ping.partial_cmp(&b.ping).unwrap())
            }
            self
        }
        pub fn result(&self) -> &Option<Vec<ServerPing>> {
            &self.ping_results
        }

        #[allow(dead_code)]
        pub fn take(&self, count: usize) -> Option<Take<std::option::Iter<'_, Vec<ServerPing>>>> {
            if self.ping_results.is_some() {
                Some(self.ping_results.iter().take(count))
            } else {
                None
            }
        }
    }
}
