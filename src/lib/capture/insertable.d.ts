// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The track the TypeScript lib's MediaStreamTrackProcessorInit leaves out.
interface MediaStreamTrackProcessorInit {
  track: MediaStreamTrack;
}
