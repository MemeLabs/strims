-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- What a player at a playout delay of $delay ms would have shown, by cell,
-- arm and the verdict's phase, from the sims' `stalls` tables (a frame's
-- stall in strims-sim, a 4 KiB chunk's in the ppspp sim: many are tiny).
-- Long form, one `part` a chart:
--
--   rate     stalls per viewer-minute and the share of viewing time
--            stalled, per cell, arm and phase; viewing time is the seconds
--            viewers were there and reading (viewer_seconds), each in the
--            phase the verdict gave that second
--   startup  the runs' startup time at the delay (rows' startup_ms_D):
--            median and p90 over seeds, per cell and arm
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
), watched AS (
    SELECT r.cell, r.arm, vs.phase, count(*) AS viewer_s
    FROM runs r JOIN with_stalls USING (k, src)
    JOIN viewer_seconds v USING (k, src)
    JOIN verdict_seconds vs USING (k, src, sec)
    WHERE v.lag_ms >= 0
    GROUP BY ALL
), stalled AS (
    -- A stall counts in the phase of the second it began in.
    SELECT r.cell, r.arm, coalesce(vs.phase, 'steady') AS phase, s.stall_ms
    FROM runs r JOIN stalls s USING (k, src)
    LEFT JOIN verdict_seconds vs ON vs.k = s.k AND vs.src = s.src AND vs.sec = s.t_ms // 1000
    WHERE s.delay_ms = $delay
), rates AS (
    SELECT w.cell, w.arm, w.phase, w.viewer_s,
           count(st.stall_ms) AS stalls, coalesce(sum(st.stall_ms), 0) AS stalled_ms
    FROM watched w LEFT JOIN stalled st USING (cell, arm, phase)
    GROUP BY ALL
), startup AS (
    SELECT r.cell, r.arm,
           quantile_cont(CASE WHEN $delay = 1250 THEN w.startup_ms_1250 ELSE w.startup_ms_4250 END, [0.5, 0.9]) AS q
    FROM runs r JOIN with_stalls USING (k, src) JOIN rows w USING (k, src)
    GROUP BY ALL
), cdf AS (
    -- Stalls in bins of a twentieth of a decade of their length.
    SELECT arm, b, count(*) AS n
    FROM (SELECT arm, round(log10(greatest(stall_ms, 1)) * 20) / 20 AS b FROM stalled)
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
