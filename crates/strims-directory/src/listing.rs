// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the server accepts as a listing, and how listings are named.
//!
//! A listing is named by its key ([`key`]): two publishes of listings with
//! the same key publish the same listing. The server accepts chat,
//! service and media listings; embeds need snippet loaders this node
//! doesn't run, so they're refused.

use prost::Message;
use strims_dao::schema::directory::listing_key;
use strims_ppspp::{SwarmId, Uri};
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::directory::{Listing, ListingContentType, ListingModeration};

use crate::DirectoryError;

/// The longest encoded listing.
pub const MAX_LISTING_BYTES: usize = 4 * 1024;
/// The longest swarm id of a media listing.
pub const MAX_SWARM_ID_BYTES: usize = 64;
/// The longest moderation category.
pub const MAX_CATEGORY_BYTES: usize = 256;

/// A listing the server accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validated {
    /// Its [`key`].
    pub key: Vec<u8>,
    pub content_type: ListingContentType,
    /// A media listing's swarm.
    pub swarm_id: Option<SwarmId>,
}

/// Checks that the server accepts `l`:
/// - it encodes to at most [`MAX_LISTING_BYTES`] ([`DirectoryError::TooLarge`]);
/// - it has content ([`DirectoryError::UnsupportedContent`]) that isn't an
///   embed ([`DirectoryError::UnsupportedEmbed`]);
/// - a media listing's swarm URI parses, with a swarm id of 1 to
///   [`MAX_SWARM_ID_BYTES`] bytes ([`DirectoryError::InvalidSwarmUri`]).
pub fn validate(l: &Listing) -> Result<Validated, DirectoryError> {
    if l.encoded_len() > MAX_LISTING_BYTES {
        return Err(DirectoryError::TooLarge);
    }
    let swarm_id = match &l.content {
        None => return Err(DirectoryError::UnsupportedContent),
        Some(Content::Embed(_)) => return Err(DirectoryError::UnsupportedEmbed),
        Some(Content::Media(m)) => {
            let uri = Uri::parse(&m.swarm_uri).map_err(|_| DirectoryError::InvalidSwarmUri)?;
            let n = uri.id.as_bytes().len();
            if n == 0 || n > MAX_SWARM_ID_BYTES {
                return Err(DirectoryError::InvalidSwarmUri);
            }
            Some(uri.id)
        }
        Some(Content::Chat(_) | Content::Service(_)) => None,
    };
    Ok(Validated {
        key: key(l),
        content_type: content_type(l),
        swarm_id,
    })
}

/// The kind of `l`'s content.
pub fn content_type(l: &Listing) -> ListingContentType {
    match &l.content {
        Some(Content::Media(_)) => ListingContentType::Media,
        Some(Content::Service(_)) => ListingContentType::Service,
        Some(Content::Embed(_)) => ListingContentType::Embed,
        Some(Content::Chat(_)) => ListingContentType::Chat,
        None => ListingContentType::Undefined,
    }
}

/// The name of `l` among a directory's listings: its record key with
/// network id 0.
pub fn key(l: &Listing) -> Vec<u8> {
    listing_key(0, Some(l))
}

/// Checks a moderation's category against [`MAX_CATEGORY_BYTES`].
pub fn validate_moderation(m: &ListingModeration) -> Result<(), DirectoryError> {
    match &m.category {
        Some(c) if c.len() > MAX_CATEGORY_BYTES => Err(DirectoryError::TooLarge),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use strims_proto::network::v1::directory::listing::{Chat, Embed, Media, Service};

    fn media(uri: &str) -> Listing {
        Listing {
            content: Some(Content::Media(Media {
                mime_type: "video/mp4".into(),
                swarm_uri: uri.into(),
            })),
        }
    }

    const URI: &str =
        "magnet:?xt=urn:ppspp:AEBAGBAFAYDQQCIKBMGA2DQPCAIREEYUCULBOGAZDINRYHI6D4QA&x.cs=1024";

    #[test]
    fn media_listings_need_a_parsable_swarm_uri() {
        let v = validate(&media(URI)).unwrap();
        assert_eq!(v.content_type, ListingContentType::Media);
        assert_eq!(v.swarm_id.unwrap().as_bytes().len(), 32);
        for bad in [
            "",
            "http://x?xt=urn:ppspp:AE",
            "magnet:?xt=urn:ppspp:",
            "magnet:?xt=foo",
        ] {
            assert_eq!(
                validate(&media(bad)),
                Err(DirectoryError::InvalidSwarmUri),
                "{bad}"
            );
        }
        let long = format!("magnet:?xt=urn:ppspp:{}", "A".repeat(120));
        assert_eq!(
            validate(&media(&long)),
            Err(DirectoryError::InvalidSwarmUri)
        );
    }

    #[test]
    fn embeds_and_empty_listings_are_refused() {
        let embed = Listing {
            content: Some(Content::Embed(Embed {
                service: 2,
                id: "x".into(),
                ..Default::default()
            })),
        };
        assert_eq!(validate(&embed), Err(DirectoryError::UnsupportedEmbed));
        assert_eq!(
            validate(&Listing::default()),
            Err(DirectoryError::UnsupportedContent)
        );
    }

    #[test]
    fn chats_and_services_are_accepted() {
        let chat = Listing {
            content: Some(Content::Chat(Chat {
                key: vec![1; 32],
                name: "c".into(),
            })),
        };
        assert_eq!(
            validate(&chat).unwrap(),
            Validated {
                key: listing_key(0, Some(&chat)),
                content_type: ListingContentType::Chat,
                swarm_id: None
            }
        );
        let service = Listing {
            content: Some(Content::Service(Service {
                r#type: "ca".into(),
                swarm_uri: "anything".into(),
            })),
        };
        assert_eq!(
            validate(&service).unwrap().content_type,
            ListingContentType::Service
        );
    }

    #[test]
    fn oversized_listings_are_refused() {
        let chat = Listing {
            content: Some(Content::Chat(Chat {
                key: vec![1; 32],
                name: "n".repeat(MAX_LISTING_BYTES),
            })),
        };
        assert_eq!(validate(&chat), Err(DirectoryError::TooLarge));
    }

    #[test]
    fn listings_with_one_key_are_one_listing() {
        let a = media(URI);
        let mut b = media(URI);
        if let Some(Content::Media(m)) = &mut b.content {
            m.mime_type = "other".into();
        }
        assert_eq!(key(&a), key(&b));
        assert_ne!(key(&a), key(&media(&URI.replace("AEBA", "AEBB"))));
    }

    #[test]
    fn categories_are_bounded() {
        let ok = ListingModeration {
            category: Some("c".repeat(MAX_CATEGORY_BYTES)),
            ..Default::default()
        };
        assert_eq!(validate_moderation(&ok), Ok(()));
        let long = ListingModeration {
            category: Some("c".repeat(MAX_CATEGORY_BYTES + 1)),
            ..Default::default()
        };
        assert_eq!(validate_moderation(&long), Err(DirectoryError::TooLarge));
    }
}
