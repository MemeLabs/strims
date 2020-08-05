#!/bin/bash
set -xe
sudo -s

wg_config=$4

cat > /etc/wireguard/wg0.conf <<EOF
$wg_config
EOF

wg addconf wg0 <(wg-quick strip wg0)
