#!/bin/bash
set -e

kubectl create namespace monitoring

#add repos
sudo helm repo add stable https://kubernetes-charts.storage.googleapis.com
sudo helm repo add loki https://grafana.github.io/loki/charts
sudo helm repo update


#generate grafana credentials
export grafana_user=$(pwgen 30 1 -s) 
export grafana_password=$(pwgen 30 1 -s)

< grafana/grafana-admin.yaml envsubst | kubectl apply -f -

unset grafana_user
unset grafana_password

#install
sudo helm install prometheus-operator -n monitoring stable/prometheus-operator -f prometheus/prom-operator-values.yaml
sudo helm install loki -n monitoring -f loki/loki-values.yaml loki/loki
sudo helm install loki-promtail -n monitoring --set 'loki.serviceName=loki' loki/promtail




