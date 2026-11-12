-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- What each arm writes besides the stream, by cell: the seed's and the
-- viewers' message bytes by kind per useful byte (rows' useful_bytes: the
-- stream's bytes the honest viewers read), and their uplinks' packets per
-- second. A kind's bytes per useful byte are the rows' totals (every
-- honest host's, the whole run): suite.py's metric where its METRICS has
-- one (job_metrics: have/useful, ack/useful, request/useful,
-- integrity/useful, wire/useful), else the rows' own fields (up_announce,
-- up_ping, up_chan_hdr, up_payload, and for state up_state, up_stream and
-- up_session), each run's weighted by its useful bytes: summed over the
-- seeds, then divided. The bins split each between the seed and the
-- viewers by their shares of its bytes there (seed_*_bytes and
-- viewer_*_bytes, stream and session counted as state); the rate (kbit/s)
-- and the packets are the bins'. Runs without bins are left out. Long
-- form:
--
--   bytes    per arm, cell, who (seed, viewers) and kind: bytes per
--            useful byte, and kbit/s over the runs' bins
--   wire     per arm, cell and who: everything their uplinks carried per
--            useful byte (wire/useful, split by the bins' *_up_bytes and
--            the acknowledgements beside them, *_up_ack_bytes and
--            *_up_sack_bytes), and kbit/s
--   packets  per arm, cell and who: packets per second
--
-- The relay and leaf split needs per-node bins (node_bins), which the
-- catalog doesn't hold yet: `viewers` is every viewer together.
WITH runs AS (
    SELECT arm_label AS arm, cell, k, src FROM experiment_jobs
    WHERE exp_id = $exp AND ($cell = '' OR cell = $cell)
), per_job AS (
    -- Each run's sums over its bins, and how long its bins span.
    SELECT r.arm, r.cell, r.k, r.src,
           (max(b.t_ms) - min(b.t_ms)) / 1000.0 AS secs,
           sum(COLUMNS('^(seed|viewer)_[a-z]+_bytes$')),
           sum(b.seed_up_packets) AS seed_up_packets, sum(b.viewer_up_packets) AS viewer_up_packets,
           -- On the wire: the uplinks' bytes and the acknowledgements beside them.
           sum(coalesce(b.seed_up_bytes, 0) + coalesce(b.seed_up_ack_bytes, 0) + coalesce(b.seed_up_sack_bytes, 0)) AS seed_wire,
           sum(coalesce(b.viewer_up_bytes, 0) + coalesce(b.viewer_up_ack_bytes, 0) + coalesce(b.viewer_up_sack_bytes, 0)) AS viewer_wire
    FROM runs r JOIN bins b USING (k, src)
    GROUP BY ALL
), per_run AS (
    -- Each run's bytes of a kind per useful byte, the rows' totals:
    -- METRICS' value where it has the kind, else the row's fields.
    SELECT r.arm, r.cell, w.useful_bytes, m.kind, m.value
    FROM (SELECT DISTINCT arm, cell, k, src FROM per_job) r JOIN rows w USING (k, src)
    JOIN (
        SELECT j.k, j.src, t.kind, j.value
        FROM job_metrics j JOIN (VALUES ('have/useful', 'have'), ('ack/useful', 'ack'), ('request/useful', 'request'),
                                        ('integrity/useful', 'integrity'), ('wire/useful', 'wire')) t(metric, kind)
          USING (metric)
        UNION ALL
        SELECT k, src, CASE field WHEN 'up_announce' THEN 'announce' WHEN 'up_ping' THEN 'ping'
                                  WHEN 'up_chan_hdr' THEN 'header' WHEN 'up_payload' THEN 'data' ELSE 'state' END,
               sum(bytes) / any_value(useful_bytes)
        FROM (SELECT k, src, useful_bytes, COLUMNS('^up_(announce|ping|chan_hdr|payload|state|stream|session)$') FROM rows)
        UNPIVOT (bytes FOR field IN (COLUMNS('^up_')))
        WHERE useful_bytes > 0
        GROUP BY ALL
    ) m USING (k, src)
    WHERE w.useful_bytes > 0 AND m.value IS NOT NULL
), totals AS (
    SELECT arm, cell, kind, sum(value * useful_bytes) / sum(useful_bytes) AS per_useful FROM per_run GROUP BY ALL
), kinds AS (
    SELECT arm, cell, secs, regexp_extract(col, '^(seed|viewer)_', 1) AS who,
           regexp_extract(col, '^(?:seed|viewer)_([a-z]+)_bytes$', 1) AS kind, bytes
    FROM (SELECT * EXCLUDE (k, src, seed_up_packets, viewer_up_packets, seed_wire, viewer_wire) FROM per_job)
    UNPIVOT (bytes FOR col IN (COLUMNS('^(seed|viewer)_[a-z]+_bytes$')))
    -- *_up_bytes is the uplink's total, every kind together.
    WHERE col NOT IN ('seed_up_bytes', 'viewer_up_bytes')
), split AS (
    -- Each kind's bins bytes by who, and their share of the kind's.
    SELECT *, bytes / nullif(sum(bytes) OVER (PARTITION BY arm, cell, kind), 0) AS share
    FROM (SELECT arm, cell, CASE who WHEN 'seed' THEN 'seed' ELSE 'viewers' END AS who,
                 CASE WHEN kind IN ('stream', 'session') THEN 'state' ELSE kind END AS kind,
                 sum(bytes) AS bytes, sum(secs) AS secs
          FROM kinds GROUP BY ALL)
), wire AS (
    SELECT *, bytes / nullif(sum(bytes) OVER (PARTITION BY arm, cell), 0) AS share
    FROM (SELECT arm, cell, 'seed' AS who, sum(seed_wire) AS bytes, sum(secs) AS secs FROM per_job GROUP BY ALL
          UNION ALL
          SELECT arm, cell, 'viewers', sum(viewer_wire), sum(secs) FROM per_job GROUP BY ALL)
)
SELECT 'bytes' AS part, s.arm, s.cell, s.who, s.kind, t.per_useful * s.share AS per_useful,
       s.bytes * 8 / 1000 / s.secs AS kbps, NULL::DOUBLE AS packets_per_s
FROM split s JOIN totals t USING (arm, cell, kind)
UNION ALL
SELECT 'wire', w.arm, w.cell, w.who, NULL, t.per_useful * w.share, w.bytes * 8 / 1000 / w.secs, NULL
FROM wire w JOIN totals t ON t.arm = w.arm AND t.cell = w.cell AND t.kind = 'wire'
UNION ALL
SELECT 'packets', arm, cell, 'seed', NULL, NULL, NULL, sum(seed_up_packets) / sum(secs) FROM per_job GROUP BY ALL
UNION ALL
SELECT 'packets', arm, cell, 'viewers', NULL, NULL, NULL, sum(viewer_up_packets) / sum(secs) FROM per_job GROUP BY ALL
ORDER BY part, arm, cell, who, kind
