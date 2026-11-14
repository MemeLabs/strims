-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- What a player at a playout delay of $delay ms would have shown the
-- honest viewers, by cell, arm and the viewer's phase, from the sims'
-- `stalls` tables (a frame's stall in strims-sim, a 4 KiB chunk's in the
-- ppspp sim: many are tiny). Long form, one `part` a chart:
--
--   rate     stalls per viewer-minute and the share of viewing time
--            stalled, per cell, arm and phase (stall_rate_of: viewing time
--            is the seconds viewers were there and reading, each in its
--            viewer's phase then, a stall in its viewer's phase of the
--            second it began in)
--   startup  the runs' honest viewers' startup time at the delay (rows'
--            startup_ms_D_honest, older rows' startup_ms_D): median and
--            p90 over seeds, per cell and arm
--   cdf      how long stalls last, per arm: the share p of the
--            experiment's stalls at the delay no longer than `value` ms
--
-- Runs from before the sims wrote stalls have none; their reads give them
-- (the run timeline plays them), where fetched.
WITH runs AS (
    SELECT cell, arm_label AS arm, seed, k, src FROM experiment_jobs WHERE exp_id = $exp
), with_stalls AS (
    -- The runs whose sim played them (their rows carry its summary).
    SELECT k, src FROM runs JOIN rows USING (k, src) WHERE stalls_per_min_1250 IS NOT NULL
), by_run AS (
    SELECT r.cell, r.arm, s.*
    FROM runs r JOIN stall_rate_of((SELECT list({'k': k, 'src': src}) FROM with_stalls), $delay) s USING (k, src)
), rates AS (
    SELECT cell, arm, phase, sum(viewer_s) AS viewer_s, sum(stalls) AS stalls, sum(stalled_ms) AS stalled_ms
    FROM by_run
    GROUP BY ALL
    HAVING sum(viewer_s) > 0
), startup AS (
    SELECT r.cell, r.arm,
           sketch_quantiles(CASE WHEN $delay = 1250 THEN coalesce(w.startup_ms_1250_honest, w.startup_ms_1250)
                              ELSE coalesce(w.startup_ms_4250_honest, w.startup_ms_4250) END, [50, 90]) AS q
    FROM runs r JOIN with_stalls USING (k, src) JOIN rows w USING (k, src)
    GROUP BY ALL
), cdf AS (
    -- Stalls in bins of a twentieth of a decade of their length.
    SELECT arm, b, count(*) AS n
    FROM (SELECT arm, round(log10(greatest(stall_ms, 1)) * 20) / 20 AS b
          FROM (SELECT arm, unnest(durations) AS stall_ms FROM by_run))
    GROUP BY ALL
)
SELECT 'rate' AS part, cell, arm, phase, 'stalls per viewer-minute' AS metric, stalls / (viewer_s / 60.0) AS value,
       stalls, viewer_s, NULL::DOUBLE AS p
FROM rates
UNION ALL
SELECT 'rate', cell, arm, phase, 'stalled share', stalled_ms / (viewer_s * 1000.0), stalls, viewer_s, NULL FROM rates
UNION ALL
SELECT 'startup', cell, arm, NULL, 'startup p50 (ms)', q[1], NULL, NULL, NULL FROM startup
UNION ALL
SELECT 'startup', cell, arm, NULL, 'startup p90 (ms)', q[2], NULL, NULL, NULL FROM startup
UNION ALL
SELECT 'cdf', NULL, arm, NULL, 'stall ms', pow(10, b), n, NULL,
       sum(n) OVER (PARTITION BY arm ORDER BY b) / sum(n) OVER (PARTITION BY arm)
FROM cdf
ORDER BY part, cell, arm, phase, p
