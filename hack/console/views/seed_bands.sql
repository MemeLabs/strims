-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- A cell and arm across its seeds: each second's honest q0.9 lag (the
-- verdict's lag_q) as quantile bands over the seeds, and the share of
-- seeds unhealthy then. Seconds count from the stream's start ($align
-- 'start'), or from each run's first event of kind $align (a sim's event:
-- depart, up_step, arrive, ...; or the verdict's: onset, collapse,
-- seed_down, seed_up, ...); a run without one is left out.
WITH runs AS (
    SELECT k, src, seed FROM experiment_jobs
    WHERE exp_id = $exp AND cell = $cell AND arm_label = $arm
), marks AS (
    SELECT k, src, sec AS t FROM runs JOIN derived_events USING (k, src) WHERE kind = $align
    UNION ALL
    SELECT k, src, t_ms / 1000 FROM runs JOIN events USING (k, src) WHERE kind = $align
), anchor AS (
    SELECT r.k, r.src, r.seed, CASE WHEN $align = 'start' THEN 0 ELSE min(m.t) END AS t0
    FROM runs r LEFT JOIN marks m USING (k, src)
    GROUP BY ALL
), secs AS (
    SELECT round(v.sec - a.t0)::BIGINT AS t, a.seed, v.lag_q, v.healthy
    FROM anchor a JOIN verdict_seconds v USING (k, src)
    WHERE a.t0 IS NOT NULL AND v.lag_q >= 0
)
SELECT t, count(*) AS seeds,
       quantile_cont(lag_q, 0.1) AS p10, quantile_cont(lag_q, 0.25) AS p25,
       quantile_cont(lag_q, 0.5) AS p50, quantile_cont(lag_q, 0.75) AS p75,
       quantile_cont(lag_q, 0.9) AS p90, max(lag_q) AS max,
       avg((NOT healthy)::INT) AS unhealthy_share,
       -- The seed whose lag is the most this second, for the tooltip.
       arg_max(seed, lag_q) AS worst_seed
FROM secs
GROUP BY t
ORDER BY t
