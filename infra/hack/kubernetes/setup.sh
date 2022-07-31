#!/usr/bin/env bash

function configure_system() {
	local hostname="$1"

	sudo hostnamectl set-hostname "${hostname}"

	DEBIAN_FRONTEND=noninteractive
	sudo apt-get update
	sudo apt-get upgrade -y

	if ! sudo grep -qa container=lxc /proc/1/environ ; then
		sudo apt autoremove -y --purge \
			snapd

		sudo rm -rf /var/cache/snapd/
		sudo apt-get clean
		sudo apt-mark hold snapd
	fi

	sudo apt-get install -y \
		apt-transport-https \
		ca-certificates \
		software-properties-common \
		curl \
		gnupg2 \
		pwgen \
		wireguard

	# Disable automatic updates
	sudo sed -i /Update/s/"1"/"0"/ /etc/apt/apt.conf.d/10periodic && sync
	echo 'APT::Periodic::Unattended-Upgrade "0";' | sudo tee -a /etc/apt/apt.conf.d/10periodic
	sudo update-ca-certificates

	# Disable swap
	sudo sed -i '/ swap / s/^\(.*\)$/#\1/g' /etc/fstab
	sudo swapoff -a

	# Wireguard requires this to configure DNS https://superuser.com/a/1544697
	sudo ln -s /usr/bin/resolvectl /usr/local/bin/resolvconf
}

function install_tools() {
	CRIO_VERSION=1.24
	VERSION_ID=$(grep VERSION_ID </etc/os-release | awk -F'=' '{print $2}' | tr -d \")
	OS=xUbuntu_$VERSION_ID
	KEYRINGS_DIR=/usr/share/keyrings
	DEBIAN_FRONTEND=noninteractive

	echo "deb [signed-by=$KEYRINGS_DIR/google-apt-key.gpg] https://apt.kubernetes.io/ kubernetes-xenial main" | sudo tee /etc/apt/sources.list.d/kubernetes.list >/dev/null

	echo "deb [signed-by=$KEYRINGS_DIR/libcontainers-archive-keyring.gpg] https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/$OS/ /" | sudo tee /etc/apt/sources.list.d/devel:kubic:libcontainers:stable.list >/dev/null
	echo "deb [signed-by=$KEYRINGS_DIR/libcontainers-crio-archive-keyring.gpg] http://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable:/cri-o:/$CRIO_VERSION/$OS/ /" | sudo tee /etc/apt/sources.list.d/devel:kubic:libcontainers:stable:cri-o:$CRIO_VERSION.list >/dev/null

	sudo mkdir -p $KEYRINGS_DIR
	curl -L https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo gpg --dearmor -o $KEYRINGS_DIR/google-apt-key.gpg
	curl -L https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/"$OS"/Release.key | sudo gpg --dearmor -o $KEYRINGS_DIR/libcontainers-archive-keyring.gpg
	curl -L https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable:/cri-o:/$CRIO_VERSION/"$OS"/Release.key | sudo gpg --dearmor -o $KEYRINGS_DIR/libcontainers-crio-archive-keyring.gpg

	sudo apt-get update
	sudo apt-get install -y \
		buildah \
		cri-o \
		cri-o-runc \
		cri-tools \
		kubelet \
		kubeadm

	sudo apt-mark hold kubelet kubeadm cri-o cri-o-runc cri-tools buildah

# According to the cri-o documentation, this is the correct config but this
# results in coredns never becoming ready because the pods are unable to reach
# the upstream dns.
# https://github.com/cri-o/cri-o/blob/c8bbae9858a084f4244cbd3bb38852d29fc0466b/tutorials/kubernetes.md#flannel-network
#	sudo tee /etc/cni/net.d/10-crio.conf <<EOF
#{
#    "cniVersion": "0.4.0",
#    "name": "crio",
#    "type": "flannel"
#}
#EOF

	sudo tee /etc/cni/net.d/100-crio-bridge.conf <<EOF
{
    "cniVersion": "0.4.0",
    "name": "crio",
    "type": "bridge",
    "bridge": "cni0",
    "isGateway": true,
    "ipMasq": true,
    "hairpinMode": true,
    "ipam": {
        "type": "host-local",
        "routes": [
            { "dst": "0.0.0.0/0" }
        ],
        "ranges": [
            [{ "subnet": "10.244.0.0/24" }]
        ]
    }
}
EOF
	sudo tee /etc/modules-load.d/crio.conf <<EOF
overlay
br_netfilter
EOF
	sudo tee /etc/sysctl.d/kubernetes.conf <<EOF
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
net.ipv4.ip_forward = 1
EOF


	if ! sudo grep -qa container=lxc /proc/1/environ ; then
		sudo modprobe overlay
		sudo modprobe br_netfilter
	fi

	sudo sysctl --system
	sudo systemctl daemon-reload
	sudo systemctl enable --now crio.service
}

function configure_firewall() {
	sudo ufw default allow outgoing
	sudo ufw default deny incoming
	sudo ufw allow ssh
	sudo ufw allow http
	sudo ufw allow https
	sudo ufw allow 1935/tcp comment 'rtmp'
	sudo ufw allow 5000 comment 'webrtc ephemeral ports'

	sudo ufw allow 51820/udp comment 'wireguard'
	sudo ufw allow in on wg0
	sudo ufw allow out on wg0
}

function start_cluster() {
	local ca_key=$1
	local public_ip=$2
	local wg_ip="10.0.0.1"

	sudo systemctl enable --now kubelet

	tee /tmp/kubeadm.yaml <<EOF
apiVersion: kubeadm.k8s.io/v1beta3
kind: InitConfiguration
localAPIEndpoint:
  advertiseAddress: ${wg_ip}
nodeRegistration:
  name: ${HOSTNAME}
  kubeletExtraArgs:
    node-ip: ${wg_ip}
    node-labels: "strims.gg/public-ip=${public_ip}"
  ignorePreflightErrors:
    - Swap
    - FileContent--proc-sys-net-bridge-bridge-nf-call-iptables
    - SystemVerification
  taints:
  - effect: NoSchedule
    key: node-role.kubernetes.io/master
  - effect: NoSchedule
    key: node-role.kubernetes.io/control-plane
certificateKey: ${ca_key}
---
apiVersion: kubeadm.k8s.io/v1beta3
kind: ClusterConfiguration
controlPlaneEndpoint: ${wg_ip}:6443
networking:
  podSubnet: 10.244.0.0/16
---
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cgroupDriver: systemd
failSwapOn: false
EOF

	sudo kubeadm init --v=5 --upload-certs --config /tmp/kubeadm.yaml

	mkdir -p "$HOME"/.kube
	sudo cp -i /etc/kubernetes/admin.conf "$HOME"/.kube/config
	sudo chown "$(id -u)":"$(id -g)" "$HOME"/.kube/config

	curl https://raw.githubusercontent.com/flannel-io/flannel/master/Documentation/kube-flannel.yml \
		| sed $'/- --kube-subnet-mgr$/a \ \ \ \ \ \ \ \ - --iface=wg0' \
		| kubectl apply -f -

	kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/v0.0.22/deploy/local-path-storage.yaml

	# curl https://raw.githubusercontent.com/helm/helm/master/scripts/get-helm-3 | bash
}

if ! command -v sudo &>/dev/null; then
	alias sudo=
fi

options=$(getopt -o nh:c:p: --long new,hostname:,ca-key:,public-ip: -- "$@")
eval set -- "$options"

NEW_CLUSTER=false
HOST_NAME=${HOSTNAME}
CA_KEY=
PUBLIC_IP=

while true; do
  case "$1" in
    -n | --new ) NEW_CLUSTER=true; shift ;;
    -h | --hostname ) HOST_NAME="$2"; shift 2 ;;
    -c | --ca-key ) CA_KEY="$2"; shift 2 ;;
    -p | --public-ip ) PUBLIC_IP="$2"; shift 2 ;;
    * ) break ;;
  esac
done

set -exo pipefail
pushd "$(/bin/pwd)" >/dev/null

if [[ "${NEW_CLUSTER}" == true ]]; then
	start_cluster "${CA_KEY}" "${PUBLIC_IP}"
else
	configure_system "${HOST_NAME}"
	install_tools
	configure_firewall
fi

popd >/dev/null
