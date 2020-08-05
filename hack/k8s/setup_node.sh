#!/bin/bash
set -ex

mkdir /etc/docker
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

sudo apt update
sudo apt install -y apt-transport-https ca-certificates software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
curl -s https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key add -
echo 'deb https://apt.kubernetes.io/ kubernetes-xenial main' | sudo tee /etc/apt/sources.list.d/kubernetes.list
sudo add-apt-repository  'deb [arch=amd64] https://download.docker.com/linux/ubuntu focal stable'
sudo apt update
# sudo apt install -y docker-ce=5:19.03.8~3-0~ubuntu-bionic kubelet=1.17.3-00 kubeadm=1.17.3-00

sudo apt install -y docker-ce kubelet kubeadm
sudo apt-mark hold docker-ce kubelet kubeadm
# sudo cp daemon.json /etc/docker/daemon.json

sudo mkdir -p /etc/systemd/system/docker.service.d
mkdir -p /var/lib/kubelet
sudo cat > kubeadm.yaml <<EOF
apiVersion: kubeadm.k8s.io/v1beta2
kind: InitConfiguration
localAPIEndpoint:
  advertiseAddress: 10.0.0.1
---
apiVersion: kubeadm.k8s.io/v1beta2
kind: ClusterConfiguration
networking:
  podSubnet: 10.244.0.0/16
---
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cgroupDriver: systemd
featureGates:
  CSIMigration: false
failSwapOn: false
EOF

# echo 'KUBELET_EXTRA_ARGS=--cgroup-driver=systemd --feature-gates='CSIMigration=false' --fail-swap-on=false' > kubelet && sudo mv kubelet /etc/default/kubelet
# sudo systemctl daemon-reload
sudo systemctl restart docker
sudo systemctl restart kubelet

sudo apt install -y wireguard

curl https://raw.githubusercontent.com/helm/helm/master/scripts/get-helm-3 | bash

--pod-network-cidr=10.244.0.0/16 --apiserver-advertise-address=10.0.0.1
