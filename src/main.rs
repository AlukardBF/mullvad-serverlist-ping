use futures::{stream::FuturesUnordered, StreamExt};
use serde_derive::Deserialize;
use std::net::IpAddr;
use winping::{AsyncPinger, Buffer};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mullvad = "https://api.mullvad.net/www/relays/all/";
    let resp: Vec<Server> = reqwest::get(mullvad).await?.json().await?;
    let servers: Vec<Server> = resp
        .iter()
        .filter(|s| s.type_field == "wireguard" && s.active == true)
        .map(|s| s.to_owned())
        .collect();
    let mut pings = ping_servers(servers).await;
    pings.sort_by(|a, b| a.ping.partial_cmp(&b.ping).unwrap());
    pings.iter().take(10).for_each(|s| println!("{:#?}", s));
    Ok(())
}

async fn ping_servers(servers: Vec<Server>) -> Vec<ServerPing> {
    servers
        .iter()
        .map(ping_server)
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await
}

async fn ping_server(server: &Server) -> ServerPing {
    let ip = server.ipv4_addr_in.parse::<IpAddr>().unwrap();
    let pinger = AsyncPinger::new();
    let mut average = 0u32;
    let num_pings = 4u32;
    for _ in 0..num_pings {
        let buf = Buffer::new();
        let ping = pinger.send(ip, buf).await;
        // average += match ping.result {
        //     Ok(rtt) => rtt,
        //     Err(_) => 99999,
        // };
        average += ping.result.unwrap_or(99999);
    }
    ServerPing {
        hostname: server.hostname.clone(),
        ping: average / num_pings,
    }
}
