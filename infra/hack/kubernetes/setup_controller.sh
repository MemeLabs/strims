#!/bin/bash
set -ex
pushd $(/bin/pwd) > /dev/null

KUB_VERSION=1.19.3-00

mkdir -p /etc/systemd/system/docker.service.d
mkdir -p /etc/docker
sudo cat > /etc/docker/daemon.json <<EOF
{
  "exec-opts": ["native.cgroupdriver=systemd"],
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "100m"
  },
  "storage-driver": "overlay2",
  "data-root": "/mnt/docker"
}
EOF

apt-get update
apt-get install -y wireguard

wg_key=`wg genkey`
wg_ip="10.0.0.1"

cat > /etc/wireguard/wg0.conf <<EOF
[Interface]
PrivateKey = $wg_key
Address = $wg_ip/24
ListenPort = 51820
EOF

wg-quick up wg0
systemctl enable wg-quick@wg0

apt-get install -y apt-transport-https ca-certificates software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add -
curl -s https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add -
echo 'deb https://apt.kubernetes.io/ kubernetes-xenial main' | tee /etc/apt/sources.list.d/kubernetes.list
add-apt-repository  'deb [arch=amd64] https://download.docker.com/linux/ubuntu focal stable'
apt-get update
apt-get install -y pwgen docker-ce=5:19.03.12~3-0~ubuntu-focal kubelet=$KUB_VERSION kubeadm=$KUB_VERSION
sudo apt-mark hold docker-ce kubelet kubeadm

sudo cat > /tmp/kubeadm.yaml <<EOF
apiVersion: kubeadm.k8s.io/v1beta2
kind: InitConfiguration
localAPIEndpoint:
  advertiseAddress: $wg_ip
nodeRegistration:
  name: controller
  kubeletExtraArgs:
    node-ip: $wg_ip
  ignorePreflightErrors:
    - Swap
    - FileContent--proc-sys-net-bridge-bridge-nf-call-iptables
    - SystemVerification
---
apiVersion: kubeadm.k8s.io/v1beta2
kind: ClusterConfiguration
networking:
  podSubnet: 10.244.0.0/16
---
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cgroupDriver: systemd
failSwapOn: false
EOF

curl https://raw.githubusercontent.com/helm/helm/master/scripts/get-helm-3 | bash

kubeadm init --config /tmp/kubeadm.yaml

mkdir -p $HOME/.kube
sudo cp -i /etc/kubernetes/admin.conf $HOME/.kube/config
sudo chown $(id -u):$(id -g) $HOME/.kube/config

kubectl apply -f /mnt/kube-flannel.yaml

kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/master/deploy/local-path-storage.yaml

popd > /dev/null
