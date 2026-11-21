# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The calibration testbed's browser node image (run.sh): the native image's
# tools, Firefox and Chromium. The harness (strims-testbed's
# testbed_browser) and the web bundle are mounted from the host, so the
# base matches the host's glibc.
#
# - Firefox is Mozilla's release .deb, from its release archive, which
#   keeps every release: its apt repo keeps only the latest few. The
#   release's SHA512SUMS doesn't list the .deb, so FIREFOX_SHA512 is the
#   .deb's own hash, and the build refuses any other file. apt installs
#   the file, resolving its dependencies, over Ubuntu's snap transitional
#   package of that name. The node hands WebRTC data channels to a worker,
#   which Firefox can from 144.
# - Firefox decodes H.264 with the system's FFmpeg (libavcodec).
# - Chromium is Chrome for Testing, Google's pinned Chromium build, with
#   the libraries its deb.deps lists (chrome-deps.sh). It runs as
#   `chromium`.
FROM ubuntu:26.04
ARG FIREFOX_VERSION=158.0.1
ARG FIREFOX_SHA512=e328bcd3a0550fc720e306a69fc516fd993523afb8d9112b89d4cc416579bd3fea834faff67a30a77dd86a28d0890c78606b284f99703cd356ef1de71023a434
ARG CHROME_VERSION=156.0.8078.47
RUN apt-get update \
 && apt-get install -y --no-install-recommends iproute2 ethtool iperf3 \
      ca-certificates curl unzip \
 && curl -fsSL -o /tmp/firefox.deb \
      "https://archive.mozilla.org/pub/firefox/releases/$FIREFOX_VERSION/linux-x86_64/en-US/firefox-$FIREFOX_VERSION.deb" \
 && echo "$FIREFOX_SHA512  /tmp/firefox.deb" | sha512sum -c - \
 && apt-get install -y --no-install-recommends /tmp/firefox.deb libavcodec62 \
 && rm -rf /var/lib/apt/lists/* /tmp/firefox.deb
COPY chrome-deps.sh /tmp/chrome-deps.sh
RUN curl -fsSL -o /tmp/chrome.zip \
      "https://storage.googleapis.com/chrome-for-testing-public/$CHROME_VERSION/linux64/chrome-linux64.zip" \
 && unzip -q /tmp/chrome.zip -d /opt \
 && rm /tmp/chrome.zip \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      $(sh /tmp/chrome-deps.sh /opt/chrome-linux64/deb.deps) \
 && ln -s /opt/chrome-linux64/chrome /usr/local/bin/chromium \
 && rm -rf /var/lib/apt/lists/* /tmp/chrome-deps.sh
RUN apt-get update \
 && apt-get install -y --no-install-recommends tcpdump \
 && rm -rf /var/lib/apt/lists/*
