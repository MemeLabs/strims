// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

type VideoPresentationMode = "inline" | "picture-in-picture" | "fullscreen";

interface HTMLVideoElement {
  webkitSetPresentationMode: (mode: VideoPresentationMode) => void;
}

interface DebugWindow {
  __strims_rtc_peer_connections__: RTCPeerConnection[];
}
