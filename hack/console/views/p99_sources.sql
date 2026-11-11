-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Where an arm's lag tail comes from: each viewer-second's lag
-- (viewer_seconds) by what the run and the viewer were doing then: the
-- viewer's phase (viewer_phases: startup, the run's or its own after it
-- joined; after_down and after_up: within 60 s of a seed uplink step;
-- steady). $cell '' takes every cell of the experiment. Long form:
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
), secs AS (
    SELECT r.arm, v.lag_ms, coalesce(p.phase, 'steady') AS phase
    FROM runs r JOIN viewer_seconds v USING (k, src)
    LEFT JOIN viewer_phases p ON p.k = v.k AND p.src = v.src AND p.viewer = v.viewer AND v.sec >= p.start
                                 AND v.sec < p.stop
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
