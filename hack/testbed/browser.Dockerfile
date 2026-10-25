# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The calibration testbed's browser node image (run.sh): the native image's
# tools, Firefox and Chromium. The harness (strims-testbed's
# testbed_browser) and the web bundle are mounted from the host, so the
# base matches the host's glibc.
#
# - Firefox is Mozilla's release .deb, pinned above Ubuntu's snap
#   transitional package. The node hands WebRTC data channels to a worker,
#   which Firefox can from 144.
# - Chromium is Chrome for Testing, Google's pinned Chromium build, with
#   the libraries its deb.deps lists (chrome-deps.sh). It runs as
#   `chromium`.
FROM ubuntu:26.04
ARG FIREFOX_VERSION=158.0.1~build1
ARG CHROME_VERSION=156.0.8078.47
RUN apt-get update \
 && apt-get install -y --no-install-recommends iproute2 ethtool iperf3 \
      ca-certificates curl unzip \
 && install -d -m 0755 /etc/apt/keyrings \
 && curl -fsSL https://packages.mozilla.org/apt/repo-signing-key.gpg \
      -o /etc/apt/keyrings/packages.mozilla.org.asc \
 && echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://packages.mozilla.org/apt mozilla main" \
      > /etc/apt/sources.list.d/mozilla.list \
 && printf 'Package: *\nPin: origin packages.mozilla.org\nPin-Priority: 1000\n' \
      > /etc/apt/preferences.d/mozilla \
 && apt-get update \
 && apt-get install -y --no-install-recommends "firefox=$FIREFOX_VERSION" \
 && rm -rf /var/lib/apt/lists/*
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
