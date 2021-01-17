#!/bin/bash
set -e

kubectl create namespace monitoring

#add repos
sudo helm repo add stable https://charts.helm.sh/stable 
sudo helm repo add prometheus-community https://prometheus-community.github.io/helm-charts

sudo helm repo add loki https://grafana.github.io/loki/charts
sudo helm repo update


#generate grafana credentials
export grafana_user=$(pwgen 30 1 -s) 
export grafana_password=$(pwgen 30 1 -s)

< grafana/grafana-admin.yaml envsubst | kubectl apply -f -

unset grafana_user
unset grafana_password

#install
#https://github.com/prometheus-community/helm-charts/tree/main/charts/kube-prometheus-stack
sudo helm install prometheus-stack -n monitoring prometheus-community/kube-prometheus-stack -f prometheus/prom-stack-values.yaml

#https://github.com/grafana/loki/tree/master/production/helm/loki
sudo helm install loki -n monitoring -f loki/loki-values.yaml loki/loki

#https://github.com/grafana/loki/tree/master/production/helm/promtail
sudo helm install loki-promtail -n monitoring --set 'loki.serviceName=loki' loki/promtail -f promtail/promtail-values.yaml




