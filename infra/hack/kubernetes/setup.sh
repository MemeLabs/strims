#!/bin/bash
set -ex
pushd $(/bin/pwd) > /dev/null

# apt update
# snap install lxd
# lxd init
# ufw allow in on lxbr0
# ufw allow out on lxbr0

container_name="strims-k8s"
container_memory="32768M"
mount_path=$HOME/data/k8s

mkdir -p $mount_path

# sudo modprobe br_netfilter
# sudo sh -c 'echo "br_netfilter" > /etc/modules-load.d/br_netfilter.conf'

# http://blog.michali.net/category/kubernetes/page/2/
cores=$(grep -c ^processor /proc/cpuinfo)
hashsize=$(( "$cores" * 16384 ))
echo -n "$hashsize" | sudo tee /sys/module/nf_conntrack/parameters/hashsize

read -r -d '' raw_lxc <<RAW_LXC || true
lxc.apparmor.profile=unconfined
lxc.mount.auto=proc:rw sys:rw cgroup:rw
lxc.cgroup.devices.allow=a
lxc.cgroup.memory.limit_in_bytes=$container_memory
lxc.cap.drop=
lxc.apparmor.allow_incomplete=1
RAW_LXC

lxc launch  \
	--config security.privileged=true \
	--config security.nesting=true \
	--config linux.kernel_modules=ip_tables,ip6_tables,netlink_diag,nf_nat,overlay,ip_vs,ip_vs_rr,ip_vs_wrr,ip_vs_sh,nf_conntrack \
	--config raw.lxc="$raw_lxc" \
ubuntu:20.04 $container_name
lxc config device add $container_name homedir disk source=$mount_path path=/mnt
lxc config device add $container_name kmsg unix-char source=/dev/kmsg path=/dev/kmsg

wait

set +e
while :
do
	lxc exec strims-k8s -- bash -c 'curl -s google.com' &> /dev/null
	ret_val=$?
	if [ $ret_val -eq 0 ]
	then
		break
	fi
	sleep 1
done
set -e

cp setup_controller.sh $mount_path
cp kube-flannel.yaml $mount_path
lxc exec strims-k8s -- bash /mnt/setup_controller.sh

# install nginx
# if we don't have an external load balancer, external ip of LoadBalancer svc has to be added manually
# lxc exec strims-k8s -- bash -c "helm repo add nginx-stable https://helm.nginx.com/stable && helm repo update"
# lxc exec strims-k8s -- bash -c "helm install --create-namespace -n ingress-nginx nginx nginx-stable/nginx-ingress"

popd > /dev/null
