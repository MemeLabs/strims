-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Two arms on the same cell and seed, on one time axis: the same audience,
-- arrivals and faults (the seed fixes them), so where the lines part is
-- where the arms differ. Long form, `arm` on every row:
--
--   lag      each second's p50, p99 and max lag across the viewers there
--   stalls   each second's stalls at a playout delay of $delay ms,
--            $min_stall_ms or longer, from the sims' `stalls` tables (none
--            for runs from before them: the run timeline plays reads)
--   verdict  each unhealthy second, and the run's collapses
WITH jobs2 AS (
    SELECT arm_label AS arm, k, src FROM experiment_jobs
    WHERE exp_id = $exp AND cell = $cell AND seed = $seed AND arm_label IN ($arm, $arm2)
), lag AS (
    SELECT j.arm, v.sec, sketch_quantile(v.lag_ms, 50) AS p50, sketch_quantile(v.lag_ms, 99) AS p99,
           max(v.lag_ms) AS max
    FROM jobs2 j JOIN viewer_seconds v USING (k, src)
    WHERE v.lag_ms >= 0
    GROUP BY ALL
)
SELECT 'lag' AS track, arm, sec::DOUBLE AS t_s, NULL::DOUBLE AS t2_s, series, value::DOUBLE AS value
FROM lag UNPIVOT (value FOR series IN (p50, p99, max))
UNION ALL
SELECT 'stalls', j.arm, s.t_ms // 1000, NULL, 'stalls', count(*)
FROM jobs2 j JOIN stalls s USING (k, src)
WHERE s.delay_ms = $delay AND s.stall_ms >= $min_stall_ms
GROUP BY ALL
UNION ALL
SELECT 'verdict', j.arm, v.sec, v.sec + 1, 'unhealthy', NULL
FROM jobs2 j JOIN verdict_seconds v USING (k, src) WHERE NOT v.healthy
UNION ALL
SELECT 'verdict', j.arm, e.start, e.stop, 'collapse', e.len
FROM jobs2 j JOIN episodes e USING (k, src) WHERE e.kind = 'collapse'
ORDER BY track, arm, series, t_s
