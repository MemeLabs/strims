-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- What a player at a playout delay of $delay ms would have shown, by cell,
-- arm and the viewer's phase, from the sims' `stalls` tables (a frame's
-- stall in strims-sim, a 4 KiB chunk's in the ppspp sim: many are tiny).
-- Long form, one `part` a chart:
--
--   rate     stalls per viewer-minute and the share of viewing time
--            stalled, per cell, arm and phase; viewing time is the seconds
--            viewers were there and reading (viewer_seconds), each in its
--            viewer's phase then (viewer_phases_of: its own startup, then
--            the verdict's phase, and past the verdict's seconds tail), a
--            stall in its viewer's phase of the second it began in
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
), spans AS (
    FROM viewer_phases_of((SELECT list({'k': k, 'src': src}) FROM with_stalls))
), watched AS (
    SELECT r.cell, r.arm, p.phase, count(*) AS viewer_s
    FROM runs r JOIN with_stalls USING (k, src)
    JOIN viewer_seconds v USING (k, src)
    JOIN spans p ON p.k = v.k AND p.src = v.src AND p.viewer = v.viewer AND v.sec >= p.start AND v.sec < p.stop
    WHERE v.lag_ms >= 0
    GROUP BY ALL
), stalled AS (
    SELECT r.cell, r.arm, p.phase, s.stall_ms
    FROM runs r JOIN with_stalls USING (k, src)
    JOIN stalls s USING (k, src)
    JOIN spans p ON p.k = s.k AND p.src = s.src AND p.viewer = s.viewer
                 AND s.t_ms // 1000 >= p.start AND s.t_ms // 1000 < p.stop
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
