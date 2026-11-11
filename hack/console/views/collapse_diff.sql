-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Two arms' collapses seed by seed: which stay on the same seeds (both)
-- and which move (only the first arm, $arm, or only the second, $arm2). A
-- ramp counts as a collapse, as the verdict counts it. Structural cells
-- (structural: every arm collapses on every seed it ran) are marked, to
-- grey them; `summary` counts each state, for the chart's header.
WITH runs AS (
    SELECT e.arm_label AS arm, e.cell, e.seed, v.collapsed
    FROM experiment_jobs e JOIN verdicts v USING (k, src)
    WHERE e.exp_id = $exp
), structural AS (
    SELECT cell FROM structural($exp)
), pairs AS (
    -- The seeds both arms ran in a cell.
    SELECT a.cell, a.seed, a.collapsed AS a, b.collapsed AS b
    FROM runs a JOIN runs b USING (cell, seed)
    WHERE a.arm = $arm AND b.arm = $arm2
), states AS (
    SELECT p.cell, p.seed,
           CASE WHEN p.a AND p.b THEN 'both'
                WHEN p.a THEN 'only first'
                WHEN p.b THEN 'only second'
                ELSE 'neither' END AS state,
           s.cell IS NOT NULL AS structural
    FROM pairs p LEFT JOIN structural s USING (cell)
)
SELECT cell, seed, $arm AS arm, state, structural,
       count(*) FILTER (WHERE state = 'both') OVER () AS both_collapsed,
       count(*) FILTER (WHERE state = 'both' AND structural) OVER () AS both_structural,
       count(*) FILTER (WHERE state = 'only first') OVER () AS only_first,
       count(*) FILTER (WHERE state = 'only second') OVER () AS only_second,
       printf('%s (first) against %s (second): both %d (%d of them structural), only %s %d, only %s %d',
              $arm, $arm2, both_collapsed, both_structural, $arm, only_first, $arm2, only_second) AS summary
FROM states
ORDER BY cell, seed
