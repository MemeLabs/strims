-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- What each arm writes besides the stream, by cell: the seed's and the
-- viewers' message bytes by kind (bins' seed_*_bytes and viewer_*_bytes,
-- summed over the run), per useful byte (rows' useful_bytes: the stream's
-- bytes the viewers had to get), and their uplinks' packets per second.
-- Summed over the seeds, then divided, as suite.py's per_useful metrics
-- are. stream and session bytes count as state. Long form:
--
--   bytes    per arm, cell, who (seed, viewers) and kind: bytes per
--            useful byte, and kbit/s over the runs' bins
--   wire     per arm, cell and who: everything their uplinks carried
--            (*_up_bytes and the acknowledgements beside them, *_up_ack_bytes
--            and *_up_sack_bytes) per useful byte, and kbit/s
--   packets  per arm, cell and who: packets per second
--
-- The relay and leaf split needs per-node bins (node_bins), which the
-- catalog doesn't hold yet: `viewers` is every viewer together.
WITH runs AS (
    SELECT arm_label AS arm, cell, k, src FROM experiment_jobs
    WHERE exp_id = $exp AND ($cell = '' OR cell = $cell)
), useful AS (
    SELECT r.arm, r.cell, sum(w.useful_bytes) AS useful_bytes
    FROM runs r JOIN rows w USING (k, src)
    GROUP BY ALL
), per_job AS (
    -- Each run's sums over its bins, and how long its bins span.
    SELECT r.arm, r.cell, r.k,
           (max(b.t_ms) - min(b.t_ms)) / 1000.0 AS secs,
           sum(COLUMNS('^(seed|viewer)_[a-z]+_bytes$')),
           sum(b.seed_up_packets) AS seed_up_packets, sum(b.viewer_up_packets) AS viewer_up_packets,
           -- On the wire: the uplinks' bytes and the acknowledgements beside them.
           sum(coalesce(b.seed_up_bytes, 0) + coalesce(b.seed_up_ack_bytes, 0) + coalesce(b.seed_up_sack_bytes, 0)) AS seed_wire,
           sum(coalesce(b.viewer_up_bytes, 0) + coalesce(b.viewer_up_ack_bytes, 0) + coalesce(b.viewer_up_sack_bytes, 0)) AS viewer_wire
    FROM runs r JOIN bins b USING (k, src)
    GROUP BY ALL
), kinds AS (
    SELECT arm, cell, secs, regexp_extract(col, '^(seed|viewer)_', 1) AS who,
           regexp_extract(col, '^(?:seed|viewer)_([a-z]+)_bytes$', 1) AS kind, bytes
    FROM (SELECT * EXCLUDE (k, seed_up_packets, viewer_up_packets, seed_wire, viewer_wire) FROM per_job)
    UNPIVOT (bytes FOR col IN (COLUMNS('^(seed|viewer)_[a-z]+_bytes$')))
    -- *_up_bytes is the uplink's total, every kind together.
    WHERE col NOT IN ('seed_up_bytes', 'viewer_up_bytes')
), totals AS (
    SELECT arm, cell, CASE who WHEN 'seed' THEN 'seed' ELSE 'viewers' END AS who,
           CASE WHEN kind IN ('stream', 'session') THEN 'state' ELSE kind END AS kind,
           sum(bytes) AS bytes, sum(secs) AS secs
    FROM kinds GROUP BY ALL
)
SELECT 'bytes' AS part, t.arm, t.cell, t.who, t.kind, t.bytes / u.useful_bytes AS per_useful,
       t.bytes * 8 / 1000 / t.secs AS kbps, NULL::DOUBLE AS packets_per_s
FROM totals t JOIN useful u USING (arm, cell)
UNION ALL
SELECT 'wire', p.arm, p.cell, 'seed', NULL, sum(p.seed_wire) / any_value(u.useful_bytes), sum(p.seed_wire) * 8 / 1000 / sum(p.secs), NULL
FROM per_job p JOIN useful u USING (arm, cell) GROUP BY ALL
UNION ALL
SELECT 'wire', p.arm, p.cell, 'viewers', NULL, sum(p.viewer_wire) / any_value(u.useful_bytes), sum(p.viewer_wire) * 8 / 1000 / sum(p.secs), NULL
FROM per_job p JOIN useful u USING (arm, cell) GROUP BY ALL
UNION ALL
SELECT 'packets', arm, cell, 'seed', NULL, NULL, NULL, sum(seed_up_packets) / sum(secs) FROM per_job GROUP BY ALL
UNION ALL
SELECT 'packets', arm, cell, 'viewers', NULL, NULL, NULL, sum(viewer_up_packets) / sum(secs) FROM per_job GROUP BY ALL
ORDER BY part, arm, cell, who, kind
