CREATE TYPE NODE_TYPE           as ENUM ('controller', 'worker');
CREATE TYPE NODE_STATE          AS ENUM ('created', 'active', 'destroyed');
CREATE TYPE WIREGUARD_PEER_TYPE AS ENUM ('node', 'external_peer');

CREATE TABLE IF NOT EXISTS "nodes" (
  "id"                BIGSERIAL PRIMARY KEY,
  "state"             NODE_STATE NOT NULL DEFAULT 'created',
  "started_at"        BIGINT NOT NULL,
  "stopped_at"        BIGINT,
  "provider_name"     TEXT NOT NULL,
  "provider_id"       TEXT NOT NULL,
  "name"              TEXT NOT NULL,
  "memory"            INTEGER NOT NULL,
  "cpus"              INTEGER NOT NULL,
  "disk"              INTEGER NOT NULL,
  "ip_v4"             TEXT NOT NULL,
  "ip_v6"             TEXT NOT NULL,
  "region_name"       TEXT NOT NULL,
  "region_lat"        DOUBLE PRECISION NOT NULL,
  "region_lng"        DOUBLE PRECISION NOT NULL,
  "sku_name"          TEXT NOT NULL,
  "sku_network_cap"   INTEGER NOT NULL,
  "sku_network_speed" INTEGER NOT NULL,
  "sku_price_monthly" REAL NOT NULL,
  "sku_price_hourly"  REAL NOT NULL,
  "wireguard_key"     TEXT NOT NULL,
  "user"              TEXT NOT NULL,
  "type"              NODE_TYPE NOT NULL,
  UNIQUE ("provider_name", "provider_id"),
  UNIQUE ("name")
);

CREATE TABLE IF NOT EXISTS "external_peers" (
  "id"                    BIGSERIAL PRIMARY KEY,
  "comment"               TEXT NOT NULL,
  "public_ip_v4"          TEXT NOT NULL,
  "wireguard_port"        INTEGER NOT NULL,
  "wireguard_private_key" TEXT NOT NULL,
  UNIQUE ("public_ip_v4")
);


CREATE TABLE IF NOT EXISTS "wireguard_ip_leases" (
  "lessee_type"           WIREGUARD_PEER_TYPE,
  "lessee_id"             BIGINT,
  "ip"                    INET NOT NULL,
  PRIMARY KEY ("lessee_type", "lessee_id"),
  UNIQUE ("ip")
);
