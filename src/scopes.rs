#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Scope {
    AppRemoteControl,
    PlaylistModifyPrivate,
    PlaylistModifyPublic,
    PlaylistReadCollaborative,
    PlaylistReadPrivate,
    Streaming,
    UgcImageUpload,
    UserFollowModify,
    UserFollowRead,
    UserLibraryModify,
    UserLibraryRead,
    UserModifyPlaybackState,
    UserReadCurrentlyPlaying,
    UserReadEmail,
    UserReadPlaybackPosition,
    UserReadPlaybackState,
    UserReadPrivate,
    UserReadRecentlyPlayed,
    UserTopRead,
}

impl Scope {
    pub fn as_str(&self) -> &str {
        use Scope::*;
        match *self {
            AppRemoteControl => "app-remote-control",
            PlaylistModifyPrivate => "playlist-modify-private",
            PlaylistModifyPublic => "playlist-modify-public",
            PlaylistReadCollaborative => "playlist-read-collaborative",
            PlaylistReadPrivate => "playlist-read-private",
            Streaming => "streaming",
            UgcImageUpload => "ugc-image-upload",
            UserFollowModify => "user-follow-modify",
            UserFollowRead => "user-follow-read",
            UserLibraryModify => "user-library-modify",
            UserLibraryRead => "user-library-read",
            UserModifyPlaybackState => "user-modify-playback-state",
            UserReadCurrentlyPlaying => "user-read-currently-playing",
            UserReadEmail => "user-read-email",
            UserReadPlaybackPosition => "user-read-playback-position",
            UserReadPlaybackState => "user-read-playback-state",
            UserReadPrivate => "user-read-private",
            UserReadRecentlyPlayed => "user-read-recently-played",
            UserTopRead => "user-top-read",
        }
    }

    pub fn to_string(scopes: &[Scope]) -> String {
        let mut value = scopes
            .iter()
            .map(|scope| scope.as_str())
            .fold(String::new(), |acc, sc| acc + sc + " ");
        let _ = value.pop();
        value
    }
}
