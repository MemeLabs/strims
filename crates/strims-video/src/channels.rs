// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The profile's video channels and their RTMP URLs.
//!
//! The profile creates and updates channels as their `Local` owner: its
//! own key, on one of its networks. A channel's snippet passes
//! [`strims_directory::snippet::check`], so the stored record stays a few
//! dozen KiB.

use strims_dao::schema::network::Networks;
use strims_dao::schema::video::{IngressConfig, VideoChannels, format_stream_key, new_channel};
use strims_dao::{Read, Singleton, Table};
use strims_directory::{Host, snippet};
use strims_kv::Kv;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::video::v1::VideoChannel;
use strims_proto::video::v1::video_channel::{Local, Owner};
use strims_rpc::Timer;

use crate::config::MAX_NETWORK_KEY_BYTES;
use crate::{VideoControl, VideoError, VideoHost};

/// The most channels the profile creates. A `List` answer holds every
/// channel, and this many at their largest fit the frontend's response.
pub const MAX_CHANNELS: usize = 64;

/// Where a broadcaster sends a channel's stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelUrl {
    /// `rtmp://<server_addr>/live/<stream_key>`.
    pub url: String,
    /// The ingress's `host:port`.
    pub server_addr: String,
    pub stream_key: String,
}

/// Checks, in `tx`, that `key` names one of the profile's networks.
async fn check_network<R: Read>(tx: &mut R, key: &[u8]) -> Result<(), VideoError> {
    if key.is_empty() || key.len() > MAX_NETWORK_KEY_BYTES {
        return Err(VideoError::NetworkNotFound);
    }
    if Networks::index_ids(tx, &Networks::BY_KEY, key)
        .await?
        .is_empty()
    {
        return Err(VideoError::NetworkNotFound);
    }
    Ok(())
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// The profile's channels, in id order.
    pub async fn list_channels(&self) -> Result<Vec<VideoChannel>, VideoError> {
        let mut tx = self.shared.store.read().await?;
        Ok(VideoChannels::get_all(&mut tx).await?)
    }

    /// The channel `id`.
    pub async fn get_channel(&self, id: u64) -> Result<VideoChannel, VideoError> {
        let mut tx = self.shared.store.read().await?;
        VideoChannels::get(&mut tx, id)
            .await?
            .ok_or(VideoError::ChannelNotFound)
    }

    /// The `Local` owner of a channel on the network `network_key`.
    fn local_owner(&self, network_key: &[u8]) -> Result<Owner, VideoError> {
        Ok(Owner::Local(Local {
            auth_key: self.shared.profile_key()?.public.clone(),
            network_key: network_key.to_vec(),
        }))
    }

    /// Creates a channel of the profile on its network `network_key`,
    /// listed with `snippet`, with a new key and token.
    ///
    /// # Errors
    ///
    /// The snippet is over its bounds, the key names none of the
    /// profile's networks, or the profile has [`MAX_CHANNELS`] channels.
    pub async fn create_channel(
        &self,
        network_key: &[u8],
        snippet: ListingSnippet,
    ) -> Result<VideoChannel, VideoError> {
        snippet::check(&snippet)?;
        let owner = self.local_owner(network_key)?;
        let s = &self.shared;
        let mut tx = s.store.write(s.timer.now()).await?;
        check_network(&mut tx, network_key).await?;
        if VideoChannels::get_all(&mut tx).await?.len() >= MAX_CHANNELS {
            return Err(VideoError::TooManyChannels);
        }
        let c = new_channel(&mut tx, owner, snippet).await?;
        let c = VideoChannels::insert(&mut tx, c).await?;
        tx.commit().await?;
        Ok(c)
    }

    /// Makes channel `id` the profile's on its network `network_key`,
    /// listed with `snippet`. The channel keeps its key and token.
    ///
    /// # Errors
    ///
    /// As [`VideoControl::create_channel`], or there's no channel `id`.
    pub async fn update_channel(
        &self,
        id: u64,
        network_key: &[u8],
        snippet: ListingSnippet,
    ) -> Result<VideoChannel, VideoError> {
        snippet::check(&snippet)?;
        let owner = self.local_owner(network_key)?;
        let s = &self.shared;
        let mut tx = s.store.write(s.timer.now()).await?;
        let mut c = VideoChannels::get(&mut tx, id)
            .await?
            .ok_or(VideoError::ChannelNotFound)?;
        check_network(&mut tx, network_key).await?;
        c.owner = Some(owner);
        c.directory_listing_snippet = Some(snippet);
        let c = VideoChannels::update(&mut tx, c).await?;
        tx.commit().await?;
        Ok(c)
    }

    /// Deletes channel `id`.
    pub async fn delete_channel(&self, id: u64) -> Result<(), VideoError> {
        let s = &self.shared;
        let mut tx = s.store.write(s.timer.now()).await?;
        if !VideoChannels::delete(&mut tx, id).await? {
            return Err(VideoError::ChannelNotFound);
        }
        tx.commit().await?;
        Ok(())
    }

    /// Where a broadcaster streams channel `id`, with a stream key the
    /// profile's key signs:
    /// - `Local`: this device's ingress, at the config's public address,
    ///   or its listen address when there's none, keyed by the channel id;
    /// - `RemoteShare`: the sharing device's ingress, keyed by the id the
    ///   share has there.
    ///
    /// # Errors
    ///
    /// There's no channel `id`, or it is a `LocalShare`: another key
    /// holder's.
    pub async fn channel_url(&self, id: u64) -> Result<ChannelUrl, VideoError> {
        let key = self.shared.profile_key()?;
        let mut tx = self.shared.store.read().await?;
        let c = VideoChannels::get(&mut tx, id)
            .await?
            .ok_or(VideoError::ChannelNotFound)?;
        let (server_addr, stream_id) = match &c.owner {
            Some(Owner::Local(_)) => {
                let config = IngressConfig::get(&mut tx).await?.unwrap_or_default();
                let addr = if config.public_server_addr.is_empty() {
                    config.server_addr
                } else {
                    config.public_server_addr
                };
                (addr, c.id)
            }
            Some(Owner::RemoteShare(r)) => (r.server_addr.clone(), r.id),
            Some(Owner::LocalShare(_)) | None => return Err(VideoError::NoStreamKey),
        };
        let stream_key =
            format_stream_key(stream_id, &c.token, key).map_err(|_| VideoError::NoProfileKey)?;
        Ok(ChannelUrl {
            url: format!("rtmp://{server_addr}/live/{stream_key}"),
            server_addr,
            stream_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use strims_directory::snippet::MAX_SNIPPET_BYTES;

    use super::*;

    #[test]
    fn the_largest_channels_fit_a_list_answer() {
        // Besides its snippet, a channel holds a key, a token and an owner
        // of two keys: well under 8 KiB.
        let channel_bytes = MAX_SNIPPET_BYTES + 8 * 1024;
        assert!(MAX_CHANNELS * channel_bytes < strims_rpc::Config::frontend().max_send_bytes);
    }
}
