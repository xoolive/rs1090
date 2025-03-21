use log::{info, warn};
use makiko::{
    ChannelConfig, Client, ClientConfig, ClientReceiver, Privkey,
    TunnelReceiver,
};
use ssh2_config::{ParseRule, SshConfig};
use std::fs::File;
use std::io::BufReader;
use tokio::net::TcpStream;

use crate::source::sshtunnel::SshTunnelIo;

pub struct TunnelledTcp {
    pub address: String,
    pub port: u16,
    pub jump: String,
}

async fn authenticate_server(
    mut client_rx: ClientReceiver,
    host: String,
    port: u16,
) {
    let mut hosts_path = dirs::home_dir().unwrap();
    hosts_path.push(".ssh");
    hosts_path.push("known_hosts");

    let hosts_data =
        std::fs::read(&hosts_path).expect("Could not read known_hosts file");

    let mut hosts_file = makiko::host_file::File::decode(hosts_data.into());

    loop {
        // Wait for the next event.
        let event = client_rx
            .recv()
            .await
            .expect("Error while receiving client event");

        // Exit the loop when the client has closed.
        let Some(event) = event else { break };

        if let makiko::ClientEvent::ServerPubkey(pubkey, accept) = event {
            info!(
                "Server pubkey type {}, fingerprint {}",
                pubkey.type_str(),
                pubkey.fingerprint()
            );

            match hosts_file.match_host_port_key(&host, port, &pubkey) {
                makiko::host_file::KeyMatch::Accepted(entries) => {
                    info!("Found the server key in known_hosts file");
                    for entry in entries.iter() {
                        info!("At line {}", entry.line());
                    }
                    accept.accept();
                }
                makiko::host_file::KeyMatch::Revoked(_entry) => {
                    panic!("The server key was revoked in known_hosts file");
                }
                makiko::host_file::KeyMatch::OtherKeys(entries) => {
                    warn!("The known_hosts file specifies other keys for this server:");
                    for entry in entries.iter() {
                        println!(
                            "At line {}, pubkey type {}, fingerprint {}",
                            entry.line(),
                            entry.pubkey().type_str(),
                            entry.pubkey().fingerprint()
                        );
                    }
                    panic!("Aborting, you might be target of a man-in-the-middle attack!");
                }
                makiko::host_file::KeyMatch::NotFound => {
                    info!("Did not find any key for this server in known_hosts file, \
                            adding it to the file");

                    accept.accept();

                    hosts_file.append_entry(
                        makiko::host_file::File::entry_builder()
                            .host_port(&host, port)
                            .key(pubkey),
                    );
                    let hosts_data = hosts_file.encode();
                    std::fs::write(&hosts_path, &hosts_data).expect(
                        "Could not write the modified known_hosts file",
                    );
                }
            }
        }
    }
}

async fn authenticate_by_private_key(
    client: &Client,
    user: &str,
    privkey: &Privkey,
) {
    let pubkey = privkey.pubkey();
    for pubkey_algo in pubkey.algos().iter().copied() {
        // Check whether this combination of a public key and algorithm would be
        // acceptable to the server.
        if client
            .check_pubkey(user.to_string(), &pubkey, pubkey_algo)
            .await
            .expect("Error when checking a public key")
        {
            // Try to authenticate with the private key
            let auth_res = client
                .auth_pubkey(user.to_string(), privkey.clone(), pubkey_algo)
                .await
                .expect("Error when trying to authenticate");

            // Deal with the possible outcomes of public key authentication.
            match auth_res {
                makiko::AuthPubkeyResult::Success => {
                    info!("We have successfully authenticated using a private key");
                    return;
                }
                makiko::AuthPubkeyResult::Failure(failure) => {
                    info!(
                        "The server rejected authentication with {:?}: {:?}",
                        pubkey_algo, failure
                    );
                }
            }
        }
    }
    panic!("The server does not accept the public key");
}

fn get_params() -> SshConfig {
    let config_path = dirs::home_dir().unwrap().join(".ssh").join("config");

    let err_msg = format!("{:?} does not exist", config_path);
    let mut reader = BufReader::new(File::open(&config_path).expect(&err_msg));

    SshConfig::default()
        .parse(
            &mut reader,
            ParseRule::ALLOW_UNKNOWN_FIELDS
                | ParseRule::ALLOW_UNSUPPORTED_FIELDS,
        )
        .unwrap_or_else(|_| {
            panic!("Failed to parse configuration file {:?}", config_path)
        })
}

fn get_default_username() -> String {
    #[cfg(target_os = "windows")]
    let username = std::env::var("USERNAME").unwrap_or_else(|_| {
        panic!("Could not determine the current Windows user name")
    });
    #[cfg(not(target_os = "windows"))]
    let username = std::env::var("USER").unwrap_or_else(|_| {
        panic!("Could not determine the current user name")
    });
    username
}

enum Io {
    Tcp(TcpStream),
    Tunnel(SshTunnelIo),
}

#[async_recursion::async_recursion]
async fn connect_server(
    server: &str,
    params: &SshConfig,
) -> Result<Client, Box<dyn std::error::Error>> {
    let server_params = params.query(server);
    let hostname = server_params.host_name.unwrap();
    let port = server_params.port.unwrap_or(22);
    let user = server_params.user.unwrap_or(get_default_username());

    let io = match server_params.unsupported_fields.get("proxyjump") {
        None => Io::Tcp(TcpStream::connect((hostname.to_owned(), port)).await?),
        Some(jump) => {
            let jump_server = jump.first().expect("No jump host specified");
            let jump_client = connect_server(jump_server, params).await?;
            let channel_config = ChannelConfig::default();
            let origin_addr = ("127.0.0.1".into(), 0);
            let (tunnel, tunnel_rx) = jump_client
                .connect_tunnel(
                    channel_config,
                    (hostname.to_owned(), port),
                    origin_addr,
                )
                .await
                .expect("Could not open a tunnel");
            Io::Tunnel(SshTunnelIo::new(tunnel, tunnel_rx))
        }
    };

    let config = ClientConfig::default();
    let (client, client_rx) = match io {
        Io::Tcp(socket) => {
            let (client, client_rx, client_fut) = Client::open(socket, config)?;
            tokio::spawn(async move {
                client_fut.await.expect("Error in client future");
            });
            (client, client_rx)
        }
        Io::Tunnel(io) => {
            let (client, client_rx, client_fut) = Client::open(io, config)?;

            tokio::spawn(async move {
                client_fut.await.expect("Error in client future");
            });
            (client, client_rx)
        }
    };

    tokio::task::spawn(authenticate_server(client_rx, hostname, port));

    // TODO try several keys
    let identity_files = server_params.identity_file.unwrap();
    let privkey = tokio::fs::read(identity_files.first().unwrap()).await?;
    let privkey = match std::env::var("SSH_PASSPHRASE").ok() {
        None => makiko::keys::decode_pem_privkey_nopass(&privkey)
            .expect("could not decode a private key from pem")
            .privkey()
            .cloned()
            .expect("Private key is encrypted"),
        Some(passphrase) => {
            makiko::keys::decode_pem_privkey(&privkey, passphrase.as_bytes())
                .expect("could not decode a private key with passphrase")
        }
    };

    authenticate_by_private_key(&client, &user, &privkey).await;

    Ok(client)
}

impl TunnelledTcp {
    pub async fn connect(
        &self,
    ) -> Result<TunnelReceiver, Box<dyn std::error::Error>> {
        let params = get_params();

        let target_client = connect_server(&self.jump, &params).await?;

        let channel_config = makiko::ChannelConfig::default();
        let connect_addr = (self.address.to_owned(), self.port);
        let origin_addr = ("0.0.0.0".into(), 0);

        let (_tunnel, tunnel_rx) = target_client
            .connect_tunnel(channel_config, connect_addr, origin_addr)
            .await
            .expect("Could not open a tunnel");

        Ok(tunnel_rx)
    }
}
