-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Where an arm's lag tail comes from: each viewer-second's lag
-- (viewer_seconds) by what the run was doing then. The phase is the
-- verdict's (startup, after_down and after_up: within 60 s of a seed
-- uplink step, steady), but a second within 10 s of a viewer's departure
-- is `departure`. $cell '' takes every cell of the experiment. Long form:
--
--   phase    per arm and phase: viewer-seconds, their share, lag quantiles,
--            the share over the playout delays (1250 and 4250 ms), and
--            `tail_share`: the share of the arm's viewer-seconds above its
--            own p99 that fall in the phase (where its p99 comes from)
--   hist     per arm and phase: viewer-seconds by lag, in bins of a
--            tenth of a decade
WITH runs AS (
    SELECT arm_label AS arm, k, src FROM experiment_jobs
    WHERE exp_id = $exp AND ($cell = '' OR cell = $cell)
), departs AS (
    SELECT k, src, t_ms / 1000 AS t FROM runs JOIN events USING (k, src) WHERE kind = 'depart'
), secs AS (
    SELECT r.arm, v.lag_ms,
           CASE WHEN EXISTS (SELECT 1 FROM departs d WHERE d.k = v.k AND d.src = v.src AND abs(d.t - v.sec) <= 10)
                THEN 'departure' ELSE coalesce(vs.phase, 'steady') END AS phase
    FROM runs r JOIN viewer_seconds v USING (k, src)
    LEFT JOIN verdict_seconds vs USING (k, src, sec)
    WHERE v.lag_ms >= 0
), arms AS (
    SELECT arm, quantile_cont(lag_ms, 0.99) AS p99, count(*) AS n FROM secs GROUP BY arm
), phases AS (
    SELECT s.arm, s.phase, count(*) AS viewer_s, any_value(a.n) AS arm_n, any_value(a.p99) AS arm_p99,
           quantile_cont(s.lag_ms, [0.5, 0.9, 0.99]) AS q, max(s.lag_ms) AS max_ms,
           avg((s.lag_ms > 1250)::INT) AS over_1250, avg((s.lag_ms > 4250)::INT) AS over_4250,
           count(*) FILTER (WHERE s.lag_ms > a.p99) AS tail
    FROM secs s JOIN arms a USING (arm)
    GROUP BY ALL
)
SELECT 'phase' AS part, arm, phase, viewer_s, viewer_s / arm_n AS share, q[1] AS p50_ms, q[2] AS p90_ms,
       q[3] AS p99_ms, max_ms, over_1250, over_4250, arm_p99 AS arm_p99_ms,
       tail / sum(tail) OVER (PARTITION BY arm) AS tail_share, NULL::DOUBLE AS lag_bin_ms
FROM phases
UNION ALL
SELECT 'hist', arm, phase, count(*), NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL,
       pow(10, round(log10(greatest(lag_ms, 1)) * 10) / 10)
FROM secs
GROUP BY ALL
ORDER BY part, arm, phase, lag_bin_ms
