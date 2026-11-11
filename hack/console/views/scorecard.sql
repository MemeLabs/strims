-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- $arm against $arm2 (the base), cell by cell, as suite.py's scorecard
-- scores them, from the catalog's scoring macros: effects (each metric's
-- geomean ratio over the per-seed pairs with its 95 % t-interval, moved
-- when it clears metric_defs' tie), collapse_test (the collapses over the
-- seeds both arms ran, with a two-sided Fisher exact p) and structural.
-- Beside each ratio, both arms' geomeans over the same pairs, whose ratio
-- is the effect. Metrics: lag p99 (lag_p99_ms) and wire bytes per useful
-- byte (wire/useful).
WITH e AS (
    SELECT cell, cell_ord, metric, n, effect AS ratio, lo, hi, mark
    FROM effects($exp, $arm2, $exp, $arm) WHERE metric IN ('lag_p99_ms', 'wire/useful')
), g AS (
    SELECT cell, metric, exp(avg(ln(a_value))) AS base_geomean, exp(avg(ln(b_value))) AS arm_geomean
    FROM pairs($exp, $arm2, $exp, $arm)
    WHERE metric IN ('lag_p99_ms', 'wire/useful') AND a_value > 0 AND b_value > 0
    GROUP BY cell, metric
)
SELECT e.cell, e.metric, e.n, e.ratio, e.lo, e.hi,
       CASE WHEN e.mark = ' ' THEN 'tie' WHEN e.ratio < 1 THEN 'lower' ELSE 'higher' END AS moved,
       g.base_geomean, g.arm_geomean, c.ka AS base_collapsed, c.kb AS arm_collapsed, c.n AS seeds,
       c.p AS fisher_p, s.cell IS NOT NULL AS structural
FROM e LEFT JOIN g USING (cell, metric)
JOIN collapse_test($exp, $arm2, $exp, $arm) c USING (cell)
LEFT JOIN structural($exp) s USING (cell)
ORDER BY e.metric, e.cell_ord
