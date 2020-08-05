#!/bin/bash
set -ex

# lxd init
# mkdir ~/container_mount

# apt update
# apt install lxd

# sudo modprobe br_netfilter
# sudo sh -c 'echo "br_netfilter" > /etc/modules-load.d/br_netfilter.conf'

cores=$(grep -c ^processor /proc/cpuinfo)
hashsize=$(( "$cores" * 16384 ))
#http://blog.michali.net/category/kubernetes/page/2/
echo -n "$hashsize" | sudo tee /sys/module/nf_conntrack/parameters/hashsize

function create_node {

	node_name=$1

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
	--config linux.kernel_modules=ip_tables,ip6_tables,netlink_diag,nf_nat,overlay \
        --config raw.lxc="${raw_lxc}" \
	ubuntu:20.04 ${node_name}
	lxc config device add ${node_name} homedir disk source=$(pwd)/data/k8s path=/mnt
	lxc config device add ${node_name} kmsg unix-char source=/dev/kmsg path=/dev/kmsg
}

function setup_wg {

	node_name=$1
	node_ip=$2

	# lxc exec $node_name -- /bin/bash -c "sudo apt install -y wireguard"
	lxc exec $node_name -- /bin/bash -c "wg genkey > private"
	lxc exec $node_name -- /bin/bash -c "ip link add wg0 type wireguard"
	lxc exec $node_name -- /bin/bash -c "ip addr add dev wg0 $node_ip/24"
	lxc exec $node_name -- /bin/bash -c "wg set wg0 listen-port 51820 private-key ./private"
	lxc exec $node_name -- /bin/bash -c "ip link set wg0 up"
	lxc exec $node_name -- /bin/bash -c "wg-quick save wg0"
	lxc exec $node_name -- /bin/bash -c "sudo systemctl enable wg-quick@wg0"
}

function connect_wg_peer {
	target_node_name=$1
	peer_node_name=$2
	peer_node_local_ip=$3
	peer_node_pub_ip=$4

	peer_key=$(lxc exec $peer_node_name -- /bin/bash -c "wg show wg0 public-key | tr -d '\n'")
	peer_port=$(lxc exec $peer_node_name -- /bin/bash -c "wg show wg0 listen-port | tr -d '\n'")

	lxc exec $target_node_name -- /bin/bash -c "wg set wg0 peer $peer_key allowed-ips $peer_node_local_ip/32 endpoint $peer_node_pub_ip:$peer_port"
}

create_node strims-k8s 32768M &

wait

set +e
while :
do
	lxc exec strims-k8s -- /bin/bash -c 'curl -s google.com' &> /dev/null
	retVal=$?
	if [ $retVal -eq 0 ]
	then
		break
	fi
	sleep 1
done
set -e

lxc exec strims-k8s -- /bin/bash -c "cd /mnt && sudo chmod +x setup_node.sh && sudo ./setup_node.sh"

setup_wg strims-k8s 10.0.0.1

#create an lxc proxy for wireguard
# lxc config device add strims-k8s wg0 proxy listen=udp:0.0.0.0:51820 connect=udp:127.0.0.1:51820

#create kubernetes strims-k8s
# lxc exec strims-k8s -- /bin/bash -c "sudo kubeadm init --node-name strims-k8s --pod-network-cidr=10.244.0.0/16 --apiserver-advertise-address=10.0.0.1 --ignore-preflight-errors=Swap,FileContent--proc-sys-net-bridge-bridge-nf-call-iptables,SystemVerification"

lxc exec strims-k8s -- /bin/bash -c "kubeadm init --node-name controller --ignore-preflight-errors=Swap,FileContent--proc-sys-net-bridge-bridge-nf-call-iptables,SystemVerification --config ./kubeadm.yaml"

#copy files for kubectl
lxc exec strims-k8s -- /bin/bash -c "mkdir -p ~/.kube && yes | sudo cp -i /etc/kubernetes/admin.conf ~/.kube/config && sudo chown $(id -u):$(id -g) ~/.kube/config"

#install the cilium networking plugin
# lxc exec strims-k8s -- /bin/bash -c "sudo helm repo add cilium https://helm.cilium.io/ && sudo helm install cilium -n kube-system cilium/cilium --version 1.6"

lxc exec strims-k8s -- /bin/bash -c "sudo kubectl apply -f https://raw.githubusercontent.com/coreos/flannel/master/Documentation/kube-flannel.yml"

#install nginx
#if we don't have an external load balancer, external ip of LoadBalancer svc has to be added manually
lxc exec strims-k8s -- /bin/bash -c "helm repo add nginx-stable https://helm.nginx.com/stable && helm repo update"
lxc exec strims-k8s -- /bin/bash -c "helm install --create-namespace -n ingress-nginx nginx nginx-stable/nginx-ingress"
