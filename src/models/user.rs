use crate::models::{ToFull, ToSimple};
use rspotify::model::{PrivateUser, PublicUser, Type as ModelType};

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
