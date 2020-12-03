use crate::models::{ToFull, ToSimple};
use rspotify::model::{PrivateUser, PublicUser, Type as ModelType, Image, Followers};
use std::collections::HashMap;

impl ToSimple for PrivateUser {
    type Simple = PublicUser;

    fn to_simple(&self) -> Self::Simple {
        PublicUser {
            display_name: self.display_name.clone(),
            external_urls: self.external_urls.clone(),
            followers: self.followers.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            images: self.images.clone().unwrap_or_else(Vec::new),
            _type: ModelType::User,
            uri: self.uri.clone(),
        }
    }

    fn into_simple(self) -> Self::Simple {
        PublicUser {
            display_name: self.display_name,
            external_urls: self.external_urls,
            followers: self.followers,
            href: self.href,
            id: self.id,
            images: self.images.unwrap_or_else(Vec::new),
            _type: ModelType::User,
            uri: self.uri,
        }
    }
}

impl ToFull for PublicUser {
    type Full = PrivateUser;

    fn to_full(&self) -> Self::Full {
        PrivateUser {
            country: None,
            display_name: self.display_name.clone(),
            email: None,
            external_urls: self.external_urls.clone(),
            explicit_content: None,
            followers: self.followers.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            images: Some(self.images.clone()),
            product: None,
            _type: ModelType::User,
            uri: self.uri.clone(),
        }
    }

    fn into_full(self) -> Self::Full {
        PrivateUser {
            country: None,
            display_name: self.display_name,
            email: None,
            external_urls: self.external_urls,
            explicit_content: None,
            followers: self.followers,
            href: self.href,
            id: self.id,
            images: Some(self.images),
            product: None,
            _type: ModelType::User,
            uri: self.uri,
        }
    }
}

#[derive(Clone, Debug, DocumentLike)]
#[pallet(tree_name = "users")]
pub struct UserModel {
    pub id: String,
    pub display_name: String,
    pub href: String,
    pub spotify_url: Option<String>,
    pub total_followers: u32,
    pub images: Vec<Image>,
}

impl From<UserModel> for PublicUser {
    fn from(model: UserModel) -> Self {
        PublicUser {
            display_name: Some(model.display_name),
            external_urls: {
                let mut map = HashMap::new();
                model.spotify_url.map(|url| {
                    map.insert("spotify".to_owned(), url);
                });
                map
            },
            followers: Some(Followers { total: model.total_followers }),
            href: model.href,
            id: model.id,
            images: model.images,
            _type: ModelType::User,
            uri: "".to_owned(),
        }
    }
}

impl From<UserModel> for PrivateUser {
    fn from(model: UserModel) -> Self {
        PrivateUser {
            country: None,
            display_name: Some(model.display_name),
            email: None,
            external_urls: {
                let mut map = HashMap::new();
                model.spotify_url.map(|url| {
                    map.insert("spotify".to_owned(), url);
                });
                map
            },
            explicit_content: None,
            followers: Some(Followers { total: model.total_followers }),
            href: model.href,
            id: model.id,
            images: Some(model.images),
            product: None,
            _type: ModelType::User,
            uri: "".to_owned(),
        }
    }
}
