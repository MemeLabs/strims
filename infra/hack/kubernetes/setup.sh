#!/usr/bin/env bash

function retry() {
	set +x

	local cmd="$*"
	local until=3
	local ready=1
	local count=1
	local sleep=1

	while [[ $ready -ne 0 ]] && [[ $count -lt $until ]]; do
		$cmd 2>&1
		ready=$(echo $?)
		((count += 1))
		sleep $sleep
		echo "Command failed, retrying..."
	done

	if [[ $ready -eq 0 ]]; then
		echo "Succeeded after $count attempts!"
	elif [[ $ready -ne 0 ]] || [[ $count -gt $until ]]; then
		echo "Failed after $count attempts!"
		exit 1
	fi

	set -x
}

function configure_system() {
	local hostname="$1"

	sudo hostnamectl set-hostname "${hostname}"

	# Disable automatic updates
	sudo sed -i /Update/s/"1"/"0"/ /etc/apt/apt.conf.d/10periodic && sync
	echo 'APT::Periodic::Unattended-Upgrade "0";' | sudo tee -a /etc/apt/apt.conf.d/10periodic
	# Fail on any errors from apt (to allow for retries)
	echo 'APT::Update::Error-Mode "any";' | sudo tee /etc/apt/apt.conf.d/10error-mode

	DEBIAN_FRONTEND=noninteractive
	retry sudo apt-get update
	retry sudo apt-get upgrade -y

	if ! sudo grep -qa container=lxc /proc/1/environ; then
		retry sudo apt autoremove -y --purge snapd

		sudo rm -rf /var/cache/snapd/
		retry sudo apt-get clean
		retry sudo apt-mark hold snapd
	fi

	retry sudo apt-get install -y \
		apt-transport-https \
		ca-certificates \
		software-properties-common \
		curl \
		gnupg2 \
		pwgen \
		wireguard

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

	sudo tee /etc/modules-load.d/crio.conf <<EOF
overlay
br_netfilter
EOF
	sudo tee /etc/sysctl.d/kubernetes.conf <<EOF
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
net.ipv4.ip_forward = 1
EOF

	if ! sudo grep -qa container=lxc /proc/1/environ; then
		sudo modprobe overlay
		sudo modprobe br_netfilter
	fi

	sudo rm -f /etc/cni/net.d/*

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
	sudo ufw allow 3478/udp comment 'coturn stun'
	sudo ufw allow in on wg0
	sudo ufw allow out on wg0
	sudo ufw allow in on cni0
	sudo ufw allow out on cni0
	sudo ufw --force enable
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
  criSocket: unix://var/run/crio/crio.sock
  kubeletExtraArgs:
    node-ip: ${wg_ip}
    node-labels: "strims.gg/public-ip=${public_ip},strims.gg/svc=leader"
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

	curl https://raw.githubusercontent.com/flannel-io/flannel/master/Documentation/kube-flannel.yml |
		sed $'/- --kube-subnet-mgr$/a \ \ \ \ \ \ \ \ - --iface=wg0' |
		kubectl apply -f -

	# https://kubernetes.io/docs/tasks/administer-cluster/dns-horizontal-autoscaling/
	kubectl patch deployment coredns -n kube-system -p '{"spec": {"replicas": 1}}'
	curl https://raw.githubusercontent.com/kubernetes/kubernetes/master/cluster/addons/dns-horizontal-autoscaler/dns-horizontal-autoscaler.yaml |
		sed 's/{{.Target}}/deployment\/coredns/' |
		kubectl apply -f -

	# TODO: fix insecure tls https://kubernetes.io/docs/tasks/administer-cluster/kubeadm/kubeadm-certs/#kubelet-serving-certs
	curl -L https://github.com/kubernetes-sigs/metrics-server/releases/download/metrics-server-helm-chart-3.8.2/components.yaml |
		sed $'/- --metric-resolution=15s$/a \ \ \ \ \ \ \ \ - --kubelet-insecure-tls' |
		kubectl apply -f -

	sudo ip link delete cni0 || :
	sudo systemctl restart crio

	# TODO: master taint is deprecated in 1.25
	kubectl taint nodes --all node-role.kubernetes.io/master:NoSchedule-
	kubectl taint nodes --all node-role.kubernetes.io/control-plane:NoSchedule-
	kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/v0.0.22/deploy/local-path-storage.yaml
	kubectl patch storageclass local-path -p '{"metadata": {"annotations":{"storageclass.kubernetes.io/is-default-class":"true"}}}'
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
	-n | --new)
		NEW_CLUSTER=true
		shift
		;;
	-h | --hostname)
		HOST_NAME="$2"
		shift 2
		;;
	-c | --ca-key)
		CA_KEY="$2"
		shift 2
		;;
	-p | --public-ip)
		PUBLIC_IP="$2"
		shift 2
		;;
	*) break ;;
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
