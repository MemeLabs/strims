#!/bin/bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The sim farm's instance boot (cloud-init user data; farm.py fills in the
# lifetime). The instance terminates when it shuts down, so the scheduled
# shutdown caps what a forgotten instance costs. Background apt work would
# take cores from the jobs.

shutdown -h +@MINUTES@
systemctl disable --now unattended-upgrades.service apt-daily.timer apt-daily-upgrade.timer || true
install -d -o ubuntu -g ubuntu /home/ubuntu/farm /home/ubuntu/farm/bin
