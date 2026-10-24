# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The calibration testbed's node image (run.sh): tc and ethtool to shape
# the container's own link. The node binary is mounted from the host, so
# the base matches the host's glibc.
FROM ubuntu:26.04
RUN apt-get update \
 && apt-get install -y --no-install-recommends iproute2 ethtool \
 && rm -rf /var/lib/apt/lists/*
