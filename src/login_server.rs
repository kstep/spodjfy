use crate::components::spotify::SpotifyCmd;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn start(spotify: Arc<Mutex<Sender<SpotifyCmd>>>) -> Result<(), Error> {
    let addr = "127.0.0.1:8888";
    let mut server = TcpListener::bind(&addr).await?;

    loop {
        let (stream, _) = server.accept().await?;
        let _ = tokio::task::spawn(handle(stream, spotify.clone())).await;
    }
}

async fn handle(mut stream: TcpStream, spotify: Arc<Mutex<Sender<SpotifyCmd>>>) {
    match process(&mut stream).await {
        Err(err) => {
            error!("error in oauth callback handler: {}", err);
            let _ = respond(stream, "HTTP/1.1 400 BAD REQUEST\r\nContent-Type: text/html\r\nContent-Length: 42\r\n\r\n<html><body><h1>Bad request!</h1></body>\r\n").await;
        }
        Ok(None) => {
            warn!("request with empty url");
            let _ = respond(stream, "HTTP/1.1 400 BAD REQUEST\r\nContent-Type: text/html\r\nContent-Length: 40\r\n\r\n<html><body><h1>Empty URL!</h1></body>\r\n").await;
        }
        Ok(Some(code)) => {
            let _ = respond(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 47\r\n\r\n<html><body><h1>Login successful!</h1></body>\r\n").await;
            info!("oauth code received");
            let _ = spotify
                .lock()
                .unwrap()
                .send(SpotifyCmd::AuthorizeUser { code });
        }
    };
}

async fn respond(mut stream: TcpStream, message: &str) -> Result<(), Error> {
    stream.write_all(message.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
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
                    line.split(" ").nth(1)
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
                    && s.chars().all(|c| match c {
                        '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '_' => true,
                        _ => false,
                    })
            })
            .map(|s| s.to_owned());

        Ok(url)
    } else {
        Ok(None)
    }
}
