use crate::services::{spotify::Spotify, SpotifyRef};
use futures_util::TryFutureExt;
use std::{
    io::{Error, ErrorKind},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    runtime::Runtime,
    sync::RwLock,
    task::JoinHandle,
};

pub struct LoginService {
    client: SpotifyRef,
}

impl LoginService {
    pub fn new(client: SpotifyRef) -> LoginService { LoginService { client } }

    pub fn spawn(self, runtime: &Runtime) -> JoinHandle<Result<!, Error>> {
        runtime.spawn(self.run().inspect_err(|error| {
            error!("login server error (no autologin is possible): {}", error);
        }))
    }

    pub async fn run(self) -> Result<!, Error> {
        let (mut server, address) = Self::bind_free_socket().await?;
        let redirect_uri = format!("http://{}/callback", address);

        info!("login server is listening at {}", redirect_uri);

        self.client.write().await.set_redirect_uri(redirect_uri);

        loop {
            let (stream, _) = server.accept().await?;
            let _ = tokio::task::spawn(Self::handle(self.client.clone(), stream)).await;
        }
    }

    async fn bind_free_socket() -> Result<(TcpListener, SocketAddr), Error> {
        const MIN_PORT: u16 = 8000;
        const MAX_PORT: u16 = 8010;

        let mut address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

        for port in MIN_PORT..=MAX_PORT {
            address.set_port(port);

            match TcpListener::bind(address).await {
                Ok(listener) => return Ok((listener, address)),
                Err(error) if error.kind() == ErrorKind::AddrInUse => (),
                Err(error) => return Err(error),
            }
        }

        Err(Error::from(ErrorKind::AddrInUse))
    }

    async fn handle(spotify: Arc<RwLock<Spotify>>, mut stream: TcpStream) {
        match Self::process(&mut stream).await {
            Err(err) => {
                error!("error in oauth callback handler: {}", err);

                let _ = Self::respond(
                    stream,
                    "HTTP/1.1 400 BAD REQUEST\r\nContent-Type: text/html\r\nContent-Length: 42\r\n\r\n<html><body><h1>Bad \
                     request!</h1></body>\r\n",
                )
                .await;
            }
            Ok(None) => {
                warn!("request with empty url");

                let _ = Self::respond(
                    stream,
                    "HTTP/1.1 400 BAD REQUEST\r\nContent-Type: text/html\r\nContent-Length: 40\r\n\r\n<html><body><h1>Empty \
                     URL!</h1></body>\r\n",
                )
                .await;
            }
            Ok(Some(code)) => {
                info!("oauth code received");

                match spotify.write().await.authorize_user(code).await {
                    Ok(_) => {
                        let _ = Self::respond(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 79\r\n\r\n<html><body><h1>Login \
                             successful!</h1><script>window.close();</script></body>\r\n",
                        )
                        .await;
                    }
                    Err(error) => {
                        let message = format!("<html><body><h1>Login error: {:?}</h1></body></html>\r\n", error);

                        let response = format!(
                            "HTTP/1.1 401 UNAUTHORIZED\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                            message.len(),
                            message
                        );

                        let _ = Self::respond(stream, &response).await;
                    }
                }
            }
        };
    }

    async fn process(stream: &mut TcpStream) -> Result<Option<String>, Error> {
        let mut buffer = [0u8; 2048];

        if stream.read(&mut buffer[..]).await? > 0 {
            let buffer = String::from_utf8_lossy(&buffer);

            if !buffer.starts_with("GET /callback?code=") {
                return Err(Error::from(ErrorKind::InvalidInput));
            }

            debug!("read data: {}", buffer);

            let url = buffer
                .lines()
                .next()
                .and_then(|line| {
                    if line.ends_with(" HTTP/1.1") {
                        line.split(' ').nth(1)
                    } else {
                        None
                    }
                })
                .map(|s| {
                    let start = 15;
                    let end = s[start..].find('&').map(|p| p + start).unwrap_or_else(|| s.len());
                    &s[start..end]
                })
                .filter(|s| !s.is_empty() && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '_')))
                .map(|s| s.to_owned());

            Ok(url)
        } else {
            Ok(None)
        }
    }

    async fn respond(mut stream: TcpStream, message: &str) -> Result<(), Error> {
        stream.write_all(message.as_bytes()).await?;
        stream.flush().await?;
        Ok(())
    }
}
