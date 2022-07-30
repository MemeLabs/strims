#!/bin/bash
set -ex

# lxd init
# mkdir ~/container_mount

# apt update
# apt install lxd

#sudo modprobe br_netfilter
#sudo sh -c 'echo "br_netfilter" > /etc/modules-load.d/br_netfilter.conf'

#cores=$(grep -c ^processor /proc/cpuinfo)
#hashsize=$(( "$cores" * 16384 ))
#http://blog.michali.net/category/kubernetes/page/2/
#echo -n "$hashsize" | sudo tee /sys/module/nf_conntrack/parameters/hashsize

function create_node {

	local node_name=$1

	read -r -d '' raw_lxc <<RAW_LXC || true
lxc.apparmor.profile=unconfined
lxc.mount.auto=proc:rw sys:rw cgroup:rw
lxc.cgroup.devices.allow=a
lxc.cgroup.memory.limit_in_bytes=$2
lxc.cap.drop=
lxc.apparmor.allow_incomplete=1
RAW_LXC

	lxc launch  \
		--config security.privileged=true \
		--config security.nesting=true \
		--config linux.kernel_modules=ip_tables,ip6_tables,netlink_diag,nf_nat,overlay,ip_vs,ip_vs_rr,ip_vs_wrr,ip_vs_sh,nf_conntrack \
		--config raw.lxc="${raw_lxc}" \
		ubuntu:20.04 ${node_name}

	lxc config device add ${node_name} homedir disk source=$(pwd)/data/k8s path=/mnt
	lxc config device add ${node_name} kmsg unix-char source=/dev/kmsg path=/dev/kmsg
	# TODO: add profile creation?
	lxc profile assign strims-k8s default,mod_br_netfilter
}

function setup_wg {

	local node_name=$1
	local node_ip=$2

	lxc exec $node_name -- /bin/bash -c "wg genkey > private"
	lxc exec $node_name -- /bin/bash -c "ip link add wg0 type wireguard"
	lxc exec $node_name -- /bin/bash -c "ip addr add dev wg0 $node_ip/24"
	lxc exec $node_name -- /bin/bash -c "wg set wg0 listen-port 51820 private-key ./private"
	lxc exec $node_name -- /bin/bash -c "ip link set wg0 up"
	lxc exec $node_name -- /bin/bash -c "wg-quick save wg0"
	lxc exec $node_name -- /bin/bash -c "sudo systemctl enable wg-quick@wg0"
}

function connect_wg_peer {
	local target_node_name=$1
	local peer_node_name=$2
	local peer_node_local_ip=$3
	local peer_node_pub_ip=$4

	local peer_key=$(lxc exec $peer_node_name -- /bin/bash -c "wg show wg0 public-key | tr -d '\n'")
	local peer_port=$(lxc exec $peer_node_name -- /bin/bash -c "wg show wg0 listen-port | tr -d '\n'")

	lxc exec $target_node_name -- /bin/bash -c "wg set wg0 peer $peer_key allowed-ips $peer_node_local_ip/32 endpoint $peer_node_pub_ip:$peer_port"
}

create_node strims-k8s 8192M
