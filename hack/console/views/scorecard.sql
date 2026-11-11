-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- $arm against $arm2 (the base), cell by cell, as suite.py's scorecard
-- scores them: paired by seed, each metric's ratio is exp of the mean of
-- ln(b / a) over the seeds where both are positive, with a 95 % t-interval
-- on that mean (suite.py's T975 table, df 17-29 taking 2.04 as it does,
-- 2.0 past 30); moved when the interval excludes 1 and |ln ratio| passes
-- ln(1 + tie) (metric_catalog's tie). Beside each ratio: both arms'
-- geomeans, the collapse counts with a two-sided Fisher exact p (a
-- hypergeometric sum, as suite.fisher), and whether the cell is structural
-- (every arm of the experiment collapses on every seed). Metrics: lag p99
-- (rows' lag_p99_ms) and wire bytes per useful byte (wire_per_useful).
WITH runs AS (
    SELECT e.cell, e.arm_label AS arm, e.seed, v.collapsed, w.lag_p99_ms, w.wire_per_useful
    FROM experiment_jobs e JOIN verdicts v USING (k, src) JOIN rows w USING (k, src)
    WHERE e.exp_id = $exp
), structural AS (
    SELECT cell FROM runs GROUP BY cell
    HAVING bool_and(collapsed) AND count(DISTINCT arm) = (SELECT count(DISTINCT arm) FROM runs)
), t975 AS (
    SELECT * FROM (VALUES (1, 12.71), (2, 4.30), (3, 3.18), (4, 2.78), (5, 2.57), (6, 2.45), (7, 2.36),
        (8, 2.31), (9, 2.26), (10, 2.23), (11, 2.20), (12, 2.18), (13, 2.16), (14, 2.14), (15, 2.13),
        (16, 2.12), (20, 2.09), (30, 2.04)) t(df, t)
), pairs AS (
    SELECT a.cell, m.metric, ln(m.b / m.a) AS d, m.a, m.b
    FROM runs a JOIN runs b USING (cell, seed),
         LATERAL (VALUES ('lag p99 (ms)', a.lag_p99_ms, b.lag_p99_ms),
                         ('wire/useful', a.wire_per_useful, b.wire_per_useful)) m(metric, a, b)
    WHERE a.arm = $arm2 AND b.arm = $arm AND m.a > 0 AND m.b > 0
), effects AS (
    SELECT cell, metric, count(*) AS n, avg(d) AS mean, stddev_samp(d) AS sd,
           exp(avg(ln(a))) AS base_geomean, exp(avg(ln(b))) AS arm_geomean
    FROM pairs GROUP BY ALL
), intervals AS (
    SELECT e.*, exp(e.mean) AS ratio,
           -- suite.py's lookup: the least t of the table's df at or past n - 1, else 2.0.
           coalesce((SELECT min(t) FROM t975 WHERE t975.df >= e.n - 1), 2.0) AS t,
           coalesce((SELECT tie FROM metric_catalog WHERE tbl = 'suite'
                     AND name = CASE e.metric WHEN 'lag p99 (ms)' THEN 'lag_p99_ms' ELSE 'wire/useful' END), 0.05) AS tie
    FROM effects e
), collapses AS (
    SELECT cell,
           count(*) FILTER (WHERE arm = $arm2) AS na, count(*) FILTER (WHERE arm = $arm2 AND collapsed) AS ka,
           count(*) FILTER (WHERE arm = $arm) AS nb, count(*) FILTER (WHERE arm = $arm AND collapsed) AS kb
    FROM runs GROUP BY cell
), fisher AS (
    -- P(x) of the hypergeometric: x of the K collapses among the base's na
    -- of the N runs; p sums every table no likelier than the one seen.
    SELECT c.cell,
           sum(exp(lnp)) FILTER (WHERE lnp <= seen + ln(1 + 1e-9)) AS p
    FROM collapses c,
         LATERAL (SELECT c.ka + c.kb AS kk, c.na + c.nb AS nn),
         LATERAL (SELECT lgamma(kk + 1) - lgamma(c.ka + 1) - lgamma(kk - c.ka + 1)
                       + lgamma(nn - kk + 1) - lgamma(c.na - c.ka + 1) - lgamma(nn - kk - c.na + c.ka + 1)
                       - lgamma(nn + 1) + lgamma(c.na + 1) + lgamma(nn - c.na + 1) AS seen),
         LATERAL (SELECT x, lgamma(kk + 1) - lgamma(x + 1) - lgamma(kk - x + 1)
                       + lgamma(nn - kk + 1) - lgamma(c.na - x + 1) - lgamma(nn - kk - c.na + x + 1)
                       - lgamma(nn + 1) + lgamma(c.na + 1) + lgamma(nn - c.na + 1) AS lnp
                  FROM range(greatest(0, c.na - (nn - kk)), least(kk, c.na) + 1) r(x))
    GROUP BY c.cell
)
SELECT i.cell, i.metric, i.n, i.ratio,
       CASE WHEN i.n > 1 THEN exp(i.mean - i.t * i.sd / sqrt(i.n)) END AS lo,
       CASE WHEN i.n > 1 THEN exp(i.mean + i.t * i.sd / sqrt(i.n)) END AS hi,
       CASE WHEN i.n > 1 AND abs(i.mean) > ln(1 + i.tie)
                 AND (exp(i.mean - i.t * i.sd / sqrt(i.n)) > 1 OR exp(i.mean + i.t * i.sd / sqrt(i.n)) < 1)
            THEN CASE WHEN i.mean < 0 THEN 'lower' ELSE 'higher' END ELSE 'tie' END AS moved,
       i.base_geomean, i.arm_geomean, c.ka AS base_collapsed, c.kb AS arm_collapsed, c.na AS base_runs,
       c.nb AS arm_runs, f.p AS fisher_p, s.cell IS NOT NULL AS structural
FROM intervals i
JOIN collapses c USING (cell) JOIN fisher f USING (cell) LEFT JOIN structural s USING (cell)
ORDER BY i.metric, i.cell
