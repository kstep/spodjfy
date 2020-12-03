use crate::services::spotify::SpotifyRef;
use futures::TryFutureExt;
use rspotify::client::ClientError;
use std::time::Duration;
use tokio::{runtime::Runtime, task::JoinHandle};

pub struct RefreshTokenService {
    client: SpotifyRef,
}

const DEFAULT_REFRESH_TOKEN_TIMEOUT: Duration = Duration::from_secs(20 * 60);

impl RefreshTokenService {
    pub fn new(client: SpotifyRef) -> RefreshTokenService { RefreshTokenService { client } }

    pub fn spawn(self, runtime: &Runtime) -> JoinHandle<Result<!, ClientError>> {
        runtime.spawn(self.run().inspect_err(|error| {
            error!("refresh token thread stopped: {:?}", error);
        }))
    }

    pub async fn run(self) -> Result<!, ClientError> {
        let mut timer = tokio::time::interval(DEFAULT_REFRESH_TOKEN_TIMEOUT);
        let spotify = self.client;

        loop {
            timer.tick().await;
            info!("refresh access token");
            spotify.write().await.refresh_user_token().await?;
        }
    }
}
