#!/bin/bash
set -ex

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

apt-get install -y apt-transport-https ca-certificates software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add -
curl -s https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add -
echo 'deb https://apt.kubernetes.io/ kubernetes-xenial main' | tee /etc/apt/sources.list.d/kubernetes.list
add-apt-repository  'deb [arch=amd64] https://download.docker.com/linux/ubuntu focal stable'
apt-get update
apt-get install -y docker-ce=5:19.03.12~3-0~ubuntu-focal kubelet=1.18.6-00 kubeadm=1.18.6-00
sudo apt-mark hold docker-ce kubelet kubeadm

curl https://raw.githubusercontent.com/helm/helm/master/scripts/get-helm-3 | bash
