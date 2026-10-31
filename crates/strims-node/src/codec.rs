// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node's codec stage: ffmpeg run as child processes on tokio, and
//! the video host that offers it to the video control.
//!
//! - [`TokioProcesses`] runs the configured ffmpeg with its standard
//!   input and output piped ([`Processes`]). A child is killed when it is
//!   dropped, and its standard error is drained as its other pipes are
//!   polled, keeping the last [`DIAGNOSTICS_BYTES`] for reports.
//! - [`probe_ffmpeg`] checks a program is an ffmpeg the stage can use: a
//!   release of [`MIN_VERSION`] or later (or a build from ffmpeg's
//!   development branch), which writes multitrack FLV, with the video
//!   encoder its settings name.
//! - [`NodeVideoHost`] is the video control's host: a [`NetVideoHost`]'s
//!   listeners, and the codec stage when the node has an ffmpeg.

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::rc::Rc;
use std::task::{Context, Poll};

use strims_video::VideoHost;
use strims_video::codec::ffmpeg::{Ffmpeg, FfmpegSettings};
use strims_video::codec::{ChildProcess, Processes};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

use crate::Net;
use crate::video_host::{NetListener, NetReader, NetVideoHost, NetWriter};

/// The oldest ffmpeg release the stage takes: the first to write
/// Enhanced RTMP multitrack FLV.
pub const MIN_VERSION: (u32, u32) = (7, 1);
/// The bytes of a child's standard error kept for reports.
pub const DIAGNOSTICS_BYTES: usize = 4096;

/// An ffmpeg a node runs, and how.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FfmpegConfig {
    /// The program, by path or by a name the `PATH` finds.
    pub program: PathBuf,
    pub settings: FfmpegSettings,
}

/// Runs a program as child processes; see the [module docs](self).
#[derive(Clone, Debug)]
pub struct TokioProcesses {
    program: Rc<PathBuf>,
}

impl TokioProcesses {
    pub fn new(program: PathBuf) -> Self {
        TokioProcesses {
            program: Rc::new(program),
        }
    }
}

impl Processes for TokioProcesses {
    type Error = io::Error;
    type Child = TokioChild;

    fn spawn(&self, args: &[String]) -> io::Result<TokioChild> {
        let mut child = Command::new(&*self.program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let missing = || io::Error::other("a pipe of the child is missing");
        Ok(TokioChild {
            stdin: Some(child.stdin.take().ok_or_else(missing)?),
            stdout: child.stdout.take().ok_or_else(missing)?,
            stderr: child.stderr.take(),
            diagnostics: VecDeque::new(),
            _child: child,
        })
    }
}

/// A running child of [`TokioProcesses`]: killed when dropped.
pub struct TokioChild {
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
    /// Until it ends.
    stderr: Option<ChildStderr>,
    /// The last of its standard error. Bound: [`DIAGNOSTICS_BYTES`].
    diagnostics: VecDeque<u8>,
    _child: Child,
}

impl TokioChild {
    /// Takes what the child wrote to its standard error so far, so its
    /// writes never wait on a full pipe.
    fn drain_stderr(&mut self, cx: &mut Context<'_>) {
        let mut buf = [0u8; 1024];
        while let Some(stderr) = &mut self.stderr {
            let mut read = ReadBuf::new(&mut buf);
            match Pin::new(stderr).poll_read(cx, &mut read) {
                Poll::Ready(Ok(())) if read.filled().is_empty() => self.stderr = None,
                Poll::Ready(Ok(())) => {
                    self.diagnostics.extend(read.filled());
                    let over = self.diagnostics.len().saturating_sub(DIAGNOSTICS_BYTES);
                    self.diagnostics.drain(..over);
                }
                Poll::Ready(Err(_)) => self.stderr = None,
                Poll::Pending => break,
            }
        }
    }
}

impl ChildProcess for TokioChild {
    type Error = io::Error;

    fn poll_write(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.drain_stderr(cx);
        match &mut self.stdin {
            Some(stdin) => Pin::new(stdin).poll_write(cx, buf),
            None => Poll::Ready(Err(io::ErrorKind::BrokenPipe.into())),
        }
    }

    fn close_input(&mut self) {
        self.stdin = None;
    }

    fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        self.drain_stderr(cx);
        let mut read = ReadBuf::new(buf);
        match Pin::new(&mut self.stdout).poll_read(cx, &mut read) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(read.filled().len())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn diagnostics(&self) -> String {
        let (a, b) = self.diagnostics.as_slices();
        String::from_utf8_lossy(&[a, b].concat()).trim().to_owned()
    }
}

/// Why a program can't be the node's ffmpeg.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("{program} didn't run: {error}")]
    Run { program: String, error: io::Error },
    #[error("{program} doesn't say it is ffmpeg")]
    NotFfmpeg { program: String },
    #[error(
        "{program} is ffmpeg {version}; the codec stage needs {}.{} or later",
        MIN_VERSION.0,
        MIN_VERSION.1
    )]
    TooOld { program: String, version: String },
    #[error("{program} has no {encoder} encoder")]
    NoEncoder { program: String, encoder: String },
}

/// Checks that `config`'s program is an ffmpeg the stage can use; see
/// the [module docs](self). Answers its version.
pub async fn probe_ffmpeg(config: &FfmpegConfig) -> Result<String, ProbeError> {
    let program = config.program.display().to_string();
    let version = run(&config.program, &["-hide_banner", "-version"]).await?;
    let Some(version) = version
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("ffmpeg version "))
        .and_then(|l| l.split_whitespace().next())
    else {
        return Err(ProbeError::NotFfmpeg { program });
    };
    if !supported(version) {
        return Err(ProbeError::TooOld {
            program,
            version: version.to_owned(),
        });
    }
    let encoder = config
        .settings
        .video_encoder
        .first()
        .cloned()
        .unwrap_or_default();
    let encoders = run(&config.program, &["-hide_banner", "-encoders"]).await?;
    if !encoders
        .lines()
        .any(|l| l.split_whitespace().nth(1) == Some(encoder.as_str()))
    {
        return Err(ProbeError::NoEncoder { program, encoder });
    }
    Ok(version.to_owned())
}

/// The standard output of `program` run with `args`.
async fn run(program: &Path, args: &[&str]) -> Result<String, ProbeError> {
    let failed = |error| ProbeError::Run {
        program: program.display().to_string(),
        error,
    };
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(failed)?;
    if !out.status.success() {
        return Err(failed(io::Error::other(format!(
            "it exited with {}",
            out.status
        ))));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Whether an ffmpeg `-version` names a version the stage takes: a
/// release (`7.1`, `n7.1.1`, `8.0.1-3ubuntu2`) of [`MIN_VERSION`] or
/// later, or a development build (`N-118325-g…`).
pub fn supported(version: &str) -> bool {
    if version.starts_with("N-") {
        return true;
    }
    let mut parts = version
        .trim_start_matches('n')
        .split(|c: char| !c.is_ascii_digit())
        .map(|p| p.parse::<u32>().ok());
    match (parts.next().flatten(), parts.next().flatten()) {
        (Some(major), minor) => (major, minor.unwrap_or(0)) >= MIN_VERSION,
        (None, _) => false,
    }
}

/// The video control's host on a native node: a [`NetVideoHost`]'s RTMP
/// listeners, and the codec stage over the node's ffmpeg, if it has one.
#[derive(Clone)]
pub struct NodeVideoHost<N: Net> {
    listeners: NetVideoHost<N>,
    codec: Option<Ffmpeg<TokioProcesses>>,
}

impl<N: Net> NodeVideoHost<N> {
    /// A host of `net`'s listeners with the codec stage of `ffmpeg`, which
    /// [`probe_ffmpeg`] checked.
    pub fn new(net: N, ffmpeg: Option<&FfmpegConfig>) -> Self {
        NodeVideoHost {
            listeners: NetVideoHost(net),
            codec: ffmpeg
                .map(|f| Ffmpeg::new(TokioProcesses::new(f.program.clone()), f.settings.clone())),
        }
    }
}

impl<N: Net> VideoHost for NodeVideoHost<N> {
    type Error = io::Error;
    type Reader = NetReader<N>;
    type Writer = NetWriter<N>;
    type Listener = NetListener<N>;
    type Codec = Ffmpeg<TokioProcesses>;

    fn rtmp_supported(&self) -> bool {
        self.listeners.rtmp_supported()
    }

    fn listen(&self, addr: &str) -> impl Future<Output = io::Result<NetListener<N>>> {
        self.listeners.listen(addr)
    }

    fn codec(&self) -> Option<Self::Codec> {
        self.codec.clone()
    }
}

#[cfg(test)]
#[path = "codec_test.rs"]
mod tests;
