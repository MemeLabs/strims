-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- An experiment's runs by cell and seed, per arm, in a run state of the
-- vocabulary's (crates/strims-wan/vocab.toml's run_states): healthy, a
-- ramp (a collapse whose honest q0.9 lag never passed DELIVERY_MAX_MS,
-- 4250 ms: viewers some seconds behind who catch up), or a collapse
-- (analyze.py's verdict: an unhealthy spell of 8 s or more past startup). A structural
-- cell (the scoring macros' structural: every arm collapses on every seed
-- it ran) says nothing about the arms, and the grid greys it.
WITH runs AS (
    SELECT e.arm_label AS arm, e.cell, e.seed, e.k, v.collapsed, v.lag_max, v.onset, v.unhealthy
    FROM experiment_jobs e JOIN verdicts v USING (k, src)
    WHERE e.exp_id = $exp
), structural AS (
    SELECT cell FROM structural($exp)
)
SELECT r.arm, r.cell, r.seed, r.k,
       CASE WHEN NOT r.collapsed THEN 'healthy'
            WHEN r.lag_max < 4250 THEN 'ramp'
            ELSE 'collapse' END AS state,
       s.cell IS NOT NULL AS structural,
       r.onset, round(r.unhealthy, 3) AS unhealthy, r.lag_max,
       -- Per arm: its collapses (ramps among them), and those outside the structural cells.
       sum(r.collapsed::INT) OVER (PARTITION BY r.arm) AS arm_collapsed,
       sum((r.collapsed AND s.cell IS NULL)::INT) OVER (PARTITION BY r.arm) AS arm_collapsed_not_structural
FROM runs r LEFT JOIN structural s USING (cell)
ORDER BY r.arm, r.cell, r.seed
