// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What the video control reports to the app.

use strims_proto::video::v1::{HlsEgressConfig, VideoChannel, VideoIngressConfig};
use strims_transfer::TransferId;

use crate::hls_egress::HlsCloseReason;

/// A change the video control saw. Subscribers get them through
/// [`VideoControl::subscribe`](crate::VideoControl::subscribe).
///
/// Record events follow the store while the control runs: each write of
/// a channel or config is reported once, in commit order. After the
/// control's store events lag, it reads the records again and reports
/// those that differ from what it last reported. A subscriber that lags
/// reads [`VideoControl::list_channels`](crate::VideoControl::list_channels)
/// and the configs again.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum VideoEvent {
    /// A channel was created or written.
    ChannelChanged(VideoChannel),
    /// A channel was deleted.
    ChannelDeleted(VideoChannel),
    /// The device's ingress config was written.
    IngressConfigChanged(VideoIngressConfig),
    /// The device's HLS egress config was written.
    HlsConfigChanged(HlsEgressConfig),
    /// An HLS stream opened ([`VideoControl::open_hls`](crate::VideoControl::open_hls)).
    HlsStreamOpened {
        transfer_id: TransferId,
        playlist_url: String,
    },
    /// An HLS stream reported by [`HlsStreamOpened`](Self::HlsStreamOpened)
    /// closed.
    HlsStreamClosed {
        transfer_id: TransferId,
        reason: HlsCloseReason,
    },
}
