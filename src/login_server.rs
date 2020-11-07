use crate::components::spotify::SpotifyCmd;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn start(spotify: Arc<Mutex<Sender<SpotifyCmd>>>) -> Result<(), std::io::Error> {
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
            let _ = stream.write_all(b"400 BAD REQUEST\nContent-Type: text/html\n\n<html><body><h1>Bad request!</h1></body>").await;
        }
        Ok(None) => {
            warn!("request with empty url");
            let _ = stream.write_all(b"400 BAD REQUEST\nContent-Type: text/html\n\n<html><body><h1>Empty URL!</h1></body>").await;
        }
        Ok(Some(code)) => {
            info!("code url received: {}", code);
            let _ = stream.write_all(b"200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Login successful!</h1></body>").await;
            let _ = spotify
                .lock()
                .unwrap()
                .send(SpotifyCmd::AuthorizeUser { code });
        }
    };
}

async fn process(stream: &mut TcpStream) -> Result<Option<String>, std::io::Error> {
    let mut buffer = [0; 1024];
    if stream.read(&mut buffer[..]).await? > 0 {
        let buffer = String::from_utf8_lossy(&buffer);
        debug!("read data: {}", buffer);
        let url = buffer
            .lines()
            .next()
            .and_then(|line| line.split(" ").nth(1))
            .map(|s| {
                let start = s.find("?code=").map(|p| p + 6).unwrap_or(0);
                let end = s[start..]
                    .find('&')
                    .map(|p| p + start)
                    .unwrap_or_else(|| s.len());
                s[start..end].to_owned()
            });

        Ok(url)
    } else {
        Ok(None)
    }
}
