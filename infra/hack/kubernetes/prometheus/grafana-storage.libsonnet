// https://github.com/prometheus-operator/kube-prometheus/issues/442#issuecomment-629410649

// In original volumes definition,  "grafana-storage" goes to emptydir.
// storageWithPVClaim replace "grafana-storage" in grafana deployment.
local k = import 'ksonnet/ksonnet.beta.3/k.libsonnet';

local pvc = k.core.v1.persistentVolumeClaim;
local grafanaPvClaimName = 'grafana-storage';
local grafanaVolName = 'grafana-storage';
local vol = k.apps.v1beta1.deployment.mixin.spec.template.spec.volumesType;

// Convert to map for easier overloading, assumes all array elements are maps having "name" field
local toNamedMap(array) = { [x.name]: x for x in array };

// Convert back to array
local toNamedArray(map) = [{ name: x } + map[x] for x in std.objectFields(map)];

local grafanaStorageWithPVClaim(storageClassName) = {
  grafana+:: {
    deployment+: {
      spec+: {
        template+: {
          spec+: {
            volumes:
            toNamedArray(toNamedMap(super.volumes) + toNamedMap([
              vol.fromPersistentVolumeClaim(grafanaVolName, grafanaPvClaimName),
            ]))
          },
        },
      },
    },
    pvc:
      pvc.new() +
      pvc.mixin.spec.withAccessModes('ReadWriteOnce') +
      pvc.mixin.spec.resources.withRequests({ storage: '5Gi' }) +
      pvc.mixin.spec.withStorageClassName(storageClassName) +
      pvc.mixin.metadata.withNamespace($._config.namespace) +
      pvc.mixin.metadata.withName(grafanaPvClaimName)
  },
};

{
  grafanaStorageWithPVClaim:: grafanaStorageWithPVClaim,
}
