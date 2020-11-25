use relm::{EventStream, Sender};
use rspotify::client::ClientError;
use std::fmt::Debug;
use std::sync::mpsc::SendError;

mod login;
pub(crate) mod spotify;

pub use login::LoginServer;
pub use spotify::{Spotify, SpotifyCmd, SpotifyProxy, SpotifyServer};

pub type ResultSender<T> = Sender<Result<T, ClientError>>;

pub trait Proxy {
    type Command;
    type Error: Debug + 'static;
    fn tell(&self, cmd: Self::Command) -> Result<(), SendError<Self::Command>>;
    fn ask<T, F, R, M>(
        &self,
        stream: EventStream<M>,
        make_command: F,
        convert_output: R,
    ) -> Result<(), SendError<Self::Command>>
    where
        F: FnOnce(Sender<Result<T, Self::Error>>) -> Self::Command + 'static,
        R: Fn(T) -> M + 'static,
        M: 'static,
    {
        let errors_stream = self.errors_stream();
        let (_, tx) = relm::Channel::<Result<T, Self::Error>>::new(move |reply| match reply {
            Ok(out) => stream.emit(convert_output(out)),
            Err(error) => {
                error!("spotify error: {:?}", error);
                errors_stream.emit(error);
            }
        });
        let cmd = make_command(tx);
        self.tell(cmd)
    }

    fn errors_stream(&self) -> EventStream<Self::Error>;
}
