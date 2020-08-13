// Reference info: documentation for https://github.com/ksonnet/ksonnet-lib can be found at http://g.bryan.dev.hepti.center
//
local k = import 'ksonnet/ksonnet.beta.3/k.libsonnet';
local grafanaPv = import 'grafana-storage.libsonnet';

local pvc = k.core.v1.persistentVolumeClaim;
local statefulSet = k.apps.v1beta2.statefulSet;
local toleration = statefulSet.mixin.spec.template.spec.tolerationsType;
local withNodeSelector = k.apps.v1beta2.deployment.mixin.spec.template.spec.withNodeSelector;

local kp =
  (import 'kube-prometheus/kube-prometheus.libsonnet') +
  grafanaPv.grafanaStorageWithPVClaim('local-storage') +
  {
    _config+:: {
      namespace: 'monitoring',
      tolerations+:: [
        {
          key: 'node-role.kubernetes.io/master',
          operator: 'Equal',
          effect: 'NoSchedule',
        }
      ],
      prometheus+:: {
        namespaces+: ['default', 'atmon'],
      },
    },

    local withTolerations() = {
      tolerations: [
        toleration.new() + (
        if std.objectHas(t, 'key') then toleration.withKey(t.key) else toleration) + (
        if std.objectHas(t, 'operator') then toleration.withOperator(t.operator) else toleration) + (
        if std.objectHas(t, 'value') then toleration.withValue(t.value) else toleration) + (
        if std.objectHas(t, 'effect') then toleration.withEffect(t.effect) else toleration),
        for t in $._config.tolerations
      ],
    },

    local persistentVolume(name, capacity, path) = {
      apiVersion: 'v1',
      kind: 'PersistentVolume',
      metadata: {
        name: name,
        namespace: 'monitoring'
      },
      spec: {
        capacity: {
          storage: capacity
        },
        accessModes: ['ReadWriteOnce'],
        persistentVolumeReclaimPolicy: 'Retain',
        storageClassName: 'local-storage',
        'local': {
          path: path
        },
        nodeAffinity: {
          required: {
            nodeSelectorTerms: [
              {
                matchExpressions: [
                  {
                    key: 'kubernetes.io/hostname',
                    operator: 'In',
                    values: ['controller']
                  }
                ]
              }
            ]
          }
        }
      }
    },

    alertmanager+:: {
      alertmanager+: {
        spec+: withTolerations() +
        {
          replicas: 1,
          nodeSelector: {
            'kubernetes.io/hostname': 'controller',
          },
        },
      },
    },

    grafana+:: {
      deployment+: {
        spec+: {
          template+: {
            spec+: withTolerations() +
            {
              nodeSelector: {
                'kubernetes.io/hostname': 'controller',
              },
              securityContext+: {
                fsGroup: 2000,
              },
            },
          },
        },
      },

      pv: persistentVolume(
        'grafana-storage',
        '5Gi',
        '/mnt/disks/grafana',
      ),
    },

    prometheus+:: {
      prometheus+: {
        spec+: withTolerations() +
        {
          replicas: 1,
          nodeSelector: {
            'kubernetes.io/hostname': 'controller',
          },
          retention: '30d',
          storage: {
            volumeClaimTemplate:
              pvc.new() +
              pvc.mixin.spec.withVolumeName('prometheus-storage') +
              pvc.mixin.spec.withAccessModes('ReadWriteOnce') +
              pvc.mixin.spec.resources.withRequests({ storage: '100Gi' }) +
              pvc.mixin.spec.withStorageClassName('local-storage'),
          },
        },
      },

      pv: persistentVolume(
        'prometheus-storage',
        '100Gi',
        '/mnt/disks/prometheus',
      ),

      clusterRole+: {
        rules+:
          local role = k.rbac.v1.role;
          local policyRule = role.rulesType;
          local rule = policyRule.new() +
                        policyRule.withApiGroups(['']) +
                        policyRule.withResources([
                          'services',
                          'endpoints',
                          'pods',
                        ]) +
                        policyRule.withVerbs(['get', 'list', 'watch']);
          [rule]
      },
    },
  };

{ ['setup/0namespace-' + name]: kp.kubePrometheus[name] for name in std.objectFields(kp.kubePrometheus) } +
{
  ['setup/prometheus-operator-' + name]: kp.prometheusOperator[name]
  for name in std.filter((function(name) name != 'serviceMonitor'), std.objectFields(kp.prometheusOperator))
} +
// serviceMonitor is separated so that it can be created after the CRDs are ready
{ 'prometheus-operator-serviceMonitor': kp.prometheusOperator.serviceMonitor } +
{ ['node-exporter-' + name]: kp.nodeExporter[name] for name in std.objectFields(kp.nodeExporter) } +
{ ['kube-state-metrics-' + name]: kp.kubeStateMetrics[name] for name in std.objectFields(kp.kubeStateMetrics) } +
{ ['alertmanager-' + name]: kp.alertmanager[name] for name in std.objectFields(kp.alertmanager) } +
{ ['prometheus-' + name]: kp.prometheus[name] for name in std.objectFields(kp.prometheus) } +
{ ['prometheus-adapter-' + name]: kp.prometheusAdapter[name] for name in std.objectFields(kp.prometheusAdapter) } +
{ ['grafana-' + name]: kp.grafana[name] for name in std.objectFields(kp.grafana) }
