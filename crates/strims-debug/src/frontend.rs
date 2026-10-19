// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's `strims.debug.v1.Debug` calls, served by [`DebugFrontend`].
//!
//! - `ReadMetrics` answers a snapshot: what the driver's [`MetricsSource`]
//!   reports, with `time` set to when it was taken and `debug` to the
//!   control's counters, mock streams and readers.
//! - `WatchMetrics` sends a snapshot at once, then one every interval: the
//!   request's, or [`Config::default_watch_interval`] for 0 or less,
//!   clamped to [`Config::min_watch_interval`] and
//!   [`Config::max_watch_interval`]. A source that fails ends the stream
//!   with its error. The call's end stops it.
//! - `GetConfig` and `SetConfig`: the device's debug config. A `SetConfig`
//!   without a config stores the default one. A config's network key is
//!   empty or names one of the profile's networks.
//! - `StartMockStream` answers the new stream's id once its swarm is
//!   created ([`DebugControl::start_mock_stream`]); `StopMockStream` of an
//!   id no mock stream has does nothing, and `ListMockStreams` lists the
//!   node's mock streams in id order.
//!
//! A refused call answers with its [`DebugError`](crate::DebugError)'s
//! message ([`DebugError::to_rpc`](crate::DebugError::to_rpc)).

use std::future::Future;
use std::rc::Rc;

use futures_util::StreamExt;
use futures_util::stream::unfold;
use strims_base::time::{Duration, Time};
use strims_directory::Host;
use strims_kv::Kv;
use strims_proto::debug::v1::{
    DebugMetrics, DebugService, GetConfigRequest, GetConfigResponse, ListMockStreamsRequest,
    ListMockStreamsResponse, Metrics, MockStream, MockStreamReader, ReadMetricsRequest,
    ReadMetricsResponse, SetConfigRequest, SetConfigResponse, StartMockStreamRequest,
    StartMockStreamResponse, StopMockStreamRequest, StopMockStreamResponse, WatchMetricsRequest,
    WatchMetricsResponse, register_debug_service,
};
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_rpc::{Router, Timer};

use crate::{Config, DebugControl, MockStreamInfo, ReaderInfo};

/// What the driver reports of the node for a metrics snapshot. The
/// frontend sets the snapshot's `time` and `debug`.
pub trait MetricsSource: 'static {
    /// The node's metrics now.
    fn metrics(&self) -> impl Future<Output = Result<Metrics, RpcError>>;
}

/// A source that reports nothing: snapshots hold only the time and the
/// debug control's metrics.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoMetrics;

impl MetricsSource for NoMetrics {
    fn metrics(&self) -> impl Future<Output = Result<Metrics, RpcError>> {
        std::future::ready(Ok(Metrics::default()))
    }
}

/// Serves the UI's debug calls on a node's debug control; see the
/// [module docs](self).
pub struct DebugFrontend<K: Kv, H: Host, T: Timer, M: MetricsSource> {
    control: DebugControl<K, H, T>,
    metrics: Rc<M>,
}

impl<K: Kv, H: Host, T: Timer, M: MetricsSource> DebugFrontend<K, H, T, M> {
    /// The frontend of `control`, with snapshots of `metrics`.
    pub fn new(control: DebugControl<K, H, T>, metrics: M) -> Self {
        DebugFrontend {
            control,
            metrics: Rc::new(metrics),
        }
    }
}

/// Serves the debug frontend of `control` on `router`, with snapshots of
/// `metrics`.
pub fn bind_frontend<K: Kv, H: Host, T: Timer, M: MetricsSource>(
    router: &Router,
    control: &DebugControl<K, H, T>,
    metrics: M,
) {
    let mut router = router.clone();
    register_debug_service(
        &mut router,
        Rc::new(DebugFrontend::new(control.clone(), metrics)),
    );
}

/// The interval between a watch's snapshots for a request of
/// `interval_ms`; see the [module docs](self).
pub(crate) fn watch_interval(c: &Config, interval_ms: i32) -> Duration {
    if interval_ms <= 0 {
        return c.default_watch_interval;
    }
    Duration::from_millis(i64::from(interval_ms)).clamp(c.min_watch_interval, c.max_watch_interval)
}

fn mock_stream(m: MockStreamInfo) -> MockStream {
    let MockStreamInfo {
        id,
        network_key,
        swarm_uri,
        bitrate_kbps,
        segment_interval_ms,
        timeout_ms,
        started_at,
        segments,
        bytes_written,
        listing_id,
    } = m;
    MockStream {
        id,
        network_key,
        swarm_uri,
        bitrate_kbps,
        segment_interval_ms,
        timeout_ms,
        started_at: started_at.unix_nanos(),
        segments,
        bytes_written,
        listing_id,
    }
}

fn reader(r: ReaderInfo) -> MockStreamReader {
    let ReaderInfo {
        network_key,
        listing_id,
        swarm_id,
        id,
        segments,
        malformed,
        discontinuities,
        latency,
        last_segment_at,
        started_at,
    } = r;
    MockStreamReader {
        network_key,
        listing_id,
        swarm_id: swarm_id.as_bytes().to_vec(),
        id,
        segments,
        malformed,
        discontinuities,
        latency: latency.map(Duration::as_nanos),
        last_segment_at: last_segment_at.map(Time::unix_nanos),
        started_at: started_at.unix_nanos(),
    }
}

/// The control's counters, mock streams and readers.
fn debug_metrics<K: Kv, H: Host, T: Timer>(control: &DebugControl<K, H, T>) -> DebugMetrics {
    let crate::Metrics {
        mock_streams_started,
        mock_streams_ended,
        segments_written,
        write_errors,
        readers_started,
        readers_ended,
        segments_read,
        read_errors,
        host_errors,
        store_errors,
        store_lags,
    } = control.metrics();
    DebugMetrics {
        mock_streams_started,
        mock_streams_ended,
        segments_written,
        write_errors,
        readers_started,
        readers_ended,
        segments_read,
        read_errors,
        host_errors,
        store_errors,
        store_lags,
        mock_streams: control
            .mock_streams()
            .into_iter()
            .map(mock_stream)
            .collect(),
        readers: control.readers().into_iter().map(reader).collect(),
    }
}

/// A snapshot of `source`, with the time and the control's metrics.
async fn snapshot<K: Kv, H: Host, T: Timer, M: MetricsSource>(
    control: &DebugControl<K, H, T>,
    source: &M,
) -> Result<Metrics, RpcError> {
    let mut m = source.metrics().await?;
    m.time = control.shared.timer.now().unix_nanos();
    m.debug = Some(debug_metrics(control));
    Ok(m)
}

/// A watch's state between snapshots: when the next is due.
struct Watch<K: Kv, H: Host, T: Timer, M: MetricsSource> {
    control: DebugControl<K, H, T>,
    source: Rc<M>,
    interval: Duration,
    next: Option<Time>,
}

impl<K: Kv, H: Host, T: Timer, M: MetricsSource> DebugService for DebugFrontend<K, H, T, M> {
    async fn read_metrics(
        &self,
        _: &Context,
        _: ReadMetricsRequest,
    ) -> Result<ReadMetricsResponse, RpcError> {
        let metrics = snapshot(&self.control, &*self.metrics).await?;
        Ok(ReadMetricsResponse {
            metrics: Some(metrics),
        })
    }

    async fn watch_metrics(
        &self,
        _: &Context,
        req: WatchMetricsRequest,
    ) -> Result<ServerStream<WatchMetricsResponse>, RpcError> {
        let w = Watch {
            control: self.control.clone(),
            source: self.metrics.clone(),
            interval: watch_interval(&self.control.shared.config, req.interval_ms),
            next: None,
        };
        let stream = unfold(Some(w), |w| async move {
            let mut w = w?;
            let timer = w.control.shared.timer.clone();
            if let Some(at) = w.next {
                timer.sleep_until(at).await;
            }
            w.next = Some(timer.now() + w.interval);
            match snapshot(&w.control, &*w.source).await {
                Ok(m) => Some((Ok(WatchMetricsResponse { metrics: Some(m) }), Some(w))),
                Err(e) => Some((Err(e), None)),
            }
        });
        Ok(stream.boxed_local())
    }

    async fn get_config(
        &self,
        _: &Context,
        _: GetConfigRequest,
    ) -> Result<GetConfigResponse, RpcError> {
        let config = self.control.config().await.map_err(|e| e.to_rpc())?;
        Ok(GetConfigResponse {
            config: Some(config),
        })
    }

    async fn set_config(
        &self,
        _: &Context,
        req: SetConfigRequest,
    ) -> Result<SetConfigResponse, RpcError> {
        let config = self
            .control
            .set_config(req.config.unwrap_or_default())
            .await
            .map_err(|e| e.to_rpc())?;
        Ok(SetConfigResponse {
            config: Some(config),
        })
    }

    async fn start_mock_stream(
        &self,
        _: &Context,
        req: StartMockStreamRequest,
    ) -> Result<StartMockStreamResponse, RpcError> {
        let id = self
            .control
            .start_mock_stream(
                req.bitrate_kbps,
                req.segment_interval_ms,
                req.timeout_ms,
                req.network_key,
            )
            .await
            .map_err(|e| e.to_rpc())?;
        Ok(StartMockStreamResponse { id })
    }

    async fn stop_mock_stream(
        &self,
        _: &Context,
        req: StopMockStreamRequest,
    ) -> Result<StopMockStreamResponse, RpcError> {
        self.control.stop_mock_stream(req.id);
        Ok(StopMockStreamResponse {})
    }

    async fn list_mock_streams(
        &self,
        _: &Context,
        _: ListMockStreamsRequest,
    ) -> Result<ListMockStreamsResponse, RpcError> {
        let mock_streams = self
            .control
            .mock_streams()
            .into_iter()
            .map(mock_stream)
            .collect();
        Ok(ListMockStreamsResponse { mock_streams })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_intervals_are_clamped() {
        let c = Config::default();
        let ms = |n: i64| Duration::from_millis(n);
        assert_eq!(watch_interval(&c, 0), ms(1000));
        assert_eq!(watch_interval(&c, -5), ms(1000));
        assert_eq!(watch_interval(&c, 1), ms(250));
        assert_eq!(watch_interval(&c, 500), ms(500));
        assert_eq!(watch_interval(&c, i32::MAX), ms(60_000));
    }
}
