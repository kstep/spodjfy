use crate::servers::spotify::Spotify;
use futures_util::TryFutureExt;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct LoginServer {
    client: Arc<Mutex<Spotify>>,
}

impl LoginServer {
    pub fn new(client: Arc<Mutex<Spotify>>) -> LoginServer {
        LoginServer { client }
    }

    pub fn spawn(self) -> JoinHandle<Result<(), Error>> {
        tokio::spawn(self.run().inspect_err(|error| {
            error!("login server error (no autologin is possible): {}", error);
        }))
    }

    pub async fn run(self) -> Result<(), Error> {
        let (mut server, address) = Self::bind_free_socket().await?;

        let redirect_uri = format!("http://{}/callback", address);
        info!("login server is listening at {}", redirect_uri);
        self.client.lock().await.set_redirect_uri(redirect_uri);

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

    async fn handle(spotify: Arc<Mutex<Spotify>>, mut stream: TcpStream) {
        match Self::process(&mut stream).await {
            Err(err) => {
                error!("error in oauth callback handler: {}", err);
                let _ = Self::respond(stream, "HTTP/1.1 400 BAD REQUEST\r\nContent-Type: text/html\r\nContent-Length: 42\r\n\r\n<html><body><h1>Bad request!</h1></body>\r\n").await;
            }
            Ok(None) => {
                warn!("request with empty url");
                let _ = Self::respond(stream, "HTTP/1.1 400 BAD REQUEST\r\nContent-Type: text/html\r\nContent-Length: 40\r\n\r\n<html><body><h1>Empty URL!</h1></body>\r\n").await;
            }
            Ok(Some(code)) => {
                info!("oauth code received");
                match spotify.lock().await.authorize_user(code).await {
                    Ok(_) => {
                        let _ = Self::respond(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 79\r\n\r\n<html><body><h1>Login successful!</h1><script>window.close();</script></body>\r\n").await;
                    }
                    Err(error) => {
                        let message = format!(
                            "<html><body><h1>Login error: {:?}</h1></body></html>\r\n",
                            error
                        );
                        let response = format!("HTTP/1.1 401 UNAUTHORIZED\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}", message.len(), message);
                        let _ = Self::respond(stream, &response).await;
                    }
                }
            }
        };
    }

    async fn process(stream: &mut TcpStream) -> Result<Option<String>, Error> {
        let mut buffer = [0u8; 640];
        if stream.read(&mut buffer[..]).await? > 0 {
            let buffer = String::from_utf8_lossy(&buffer);
            if !buffer.starts_with("GET ") {
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
                    let start = s.find("?code=").map(|p| p + 6).unwrap_or(0);
                    let end = s[start..]
                        .find('&')
                        .map(|p| p + start)
                        .unwrap_or_else(|| s.len());
                    &s[start..end]
                })
                .filter(|s| {
                    !s.is_empty()
                        && s.chars()
                            .all(|c| matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '_'))
                })
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
