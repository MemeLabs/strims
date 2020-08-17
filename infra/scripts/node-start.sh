#!/bin/bash
set -xe

node_user=$1
node_addr=$2
node_key_path=$3
wg_ip = $4
wg_config=$5
node_name=$6

read -a join_cmd <<< `kubeadm token create --print-join-command | tr -d '\n'`
k8s_api_server_endpoint=${join_cmd[2]}
k8s_token=${join_cmd[4]}
k8s_ca_cert_hash=${join_cmd[6]}

ssh -T $node_user@$node_addr -i $node_key_path -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no <<CMD
#!/bin/bash
set -ex
sudo -s

ufw default allow outgoing
ufw default deny incoming
ufw allow ssh
ufw allow http
ufw allow https
ufw allow 1935/tcp comment 'rtmp'
ufw allow 50000:60000/udp comment 'webrtc ephemeral ports'

ufw allow 51820/udp comment 'wireguard'
ufw allow in on wg0
ufw allow out on wg0

hostname $node_name
echo "$node_name" > /etc/hostname
# TODO: add /etc/hosts entries for 127.0.0.1 and ::1

mkdir -p /etc/systemd/system/docker.service.d
mkdir -p /etc/docker
cat > /etc/docker/daemon.json <<EOF
{
  "exec-opts": ["native.cgroupdriver=systemd"],
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "100m"
  },
  "storage-driver": "overlay2"
}
EOF

apt-get update
apt-get install -y wireguard

cat > /etc/wireguard/wg0.conf <<EOF
$wg_config
EOF

wg-quick up wg0
systemctl enable wg-quick@wg0

apt-get install -y apt-transport-https ca-certificates software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add -
curl -s https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add -
echo 'deb https://apt.kubernetes.io/ kubernetes-xenial main' | tee /etc/apt/sources.list.d/kubernetes.list
add-apt-repository  'deb [arch=amd64] https://download.docker.com/linux/ubuntu focal stable'
apt-get update
apt-get install -y docker-ce=5:19.03.12~3-0~ubuntu-focal kubelet=1.18.6-00 kubeadm=1.18.6-00
apt-mark hold docker-ce kubelet kubeadm

cat > /tmp/kubeadm.yml <<EOF
apiVersion: kubeadm.k8s.io/v1beta2
kind: JoinConfiguration
discovery:
  bootstrapToken:
    apiServerEndpoint: $k8s_api_server_endpoint
    token: $k8s_token
    caCertHashes: [$k8s_ca_cert_hash]
nodeRegistration:
  name: $node_name
  kubeletExtraArgs:
    node-ip: $wg_ip
EOF

kubeadm join --config=/tmp/kubeadm.yml
CMD
