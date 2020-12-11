#!/usr/bin/env bash
set -xe

node_user=$1
node_addr=$2
node_key_path=$3
wg_config=$4

# shellcheck disable=SC2087
ssh -T "$node_user"@"$node_addr" -i "$node_key_path" -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no <<CMD
#!/bin/bash
set -ex
sudo -s

cat > /etc/wireguard/wg0.conf <<EOF
$wg_config
EOF

wg syncconf wg0 <(wg-quick strip wg0)
CMD
