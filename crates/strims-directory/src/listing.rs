// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the server accepts as a listing, and how listings are named.
//!
//! A listing is named by its key ([`key`]): two publishes of listings with
//! the same key publish the same listing. The server accepts chat,
//! service, media and embed listings; an embed's service must also be one
//! the server enables, which the server checks.

use prost::Message;
use strims_dao::schema::directory::listing_key;
use strims_ppspp::{SwarmId, Uri};
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::directory::{Listing, ListingContentType, ListingModeration};

use crate::DirectoryError;
use crate::embed::{self, EmbedService};

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
    /// An embed listing's service.
    pub embed: Option<EmbedService>,
}

/// Checks that the server accepts `l`:
/// - it encodes to at most [`MAX_LISTING_BYTES`] ([`DirectoryError::TooLarge`]);
/// - it has content ([`DirectoryError::UnsupportedContent`]);
/// - a media listing's swarm URI parses, with a swarm id of 1 to
///   [`MAX_SWARM_ID_BYTES`] bytes ([`DirectoryError::InvalidSwarmUri`]);
/// - an embed names a known service ([`DirectoryError::UnsupportedEmbed`])
///   and an id of its form ([`embed::validate_id`]).
pub fn validate(l: &Listing) -> Result<Validated, DirectoryError> {
    if l.encoded_len() > MAX_LISTING_BYTES {
        return Err(DirectoryError::TooLarge);
    }
    let (swarm_id, embed_service) = match &l.content {
        None => return Err(DirectoryError::UnsupportedContent),
        Some(Content::Embed(e)) => {
            let service =
                EmbedService::from_proto(e.service).ok_or(DirectoryError::UnsupportedEmbed)?;
            embed::validate_id(service, &e.id)?;
            (None, Some(service))
        }
        Some(Content::Media(m)) => {
            let uri = Uri::parse(&m.swarm_uri).map_err(|_| DirectoryError::InvalidSwarmUri)?;
            let n = uri.id.as_bytes().len();
            if n == 0 || n > MAX_SWARM_ID_BYTES {
                return Err(DirectoryError::InvalidSwarmUri);
            }
            (Some(uri.id), None)
        }
        Some(Content::Chat(_) | Content::Service(_)) => (None, None),
    };
    Ok(Validated {
        key: key(l),
        content_type: content_type(l),
        swarm_id,
        embed: embed_service,
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

    fn embed(service: i32, id: &str) -> Listing {
        Listing {
            content: Some(Content::Embed(Embed {
                service,
                id: id.into(),
                ..Default::default()
            })),
        }
    }

    #[test]
    fn embeds_need_a_known_service_and_an_id_of_its_form() {
        let v = validate(&embed(2, "destiny")).unwrap();
        assert_eq!(v.content_type, ListingContentType::Embed);
        assert_eq!(v.embed, Some(EmbedService::TwitchStream));
        assert_eq!(v.swarm_id, None);
        assert_eq!(
            validate(&embed(4, "DDTGlyJVNVI")).unwrap().embed,
            Some(EmbedService::YouTube)
        );
        for service in [0, 5, -1] {
            assert_eq!(
                validate(&embed(service, "x")),
                Err(DirectoryError::UnsupportedEmbed)
            );
        }
        for (service, id) in [(2, "a&b"), (3, "x"), (4, "short"), (1, "")] {
            assert_eq!(
                validate(&embed(service, id)),
                Err(DirectoryError::InvalidEmbedId),
                "{service} {id}"
            );
        }
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
                swarm_id: None,
                embed: None,
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
