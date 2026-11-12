-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- One run's push trees, from the sims' `parents` table (each change of a
-- node's open parent of a stream: hybrid push mode only, so empty for
-- other runs). The seed is node 0; a node's depth in a stream is its hops
-- from the seed along its parents, NULL while it has none or its parents
-- don't reach the seed. Long form:
--
--   depth     each second: the mean and the most depth over the nodes and
--             streams with a parent, and the share of them without one
--   change    each second: the parent changes, by reason (open, switch,
--             lost, prune, shed)
--   snapshot  at $t s: each node's depth in each stream, and its parent
WITH RECURSIVE job AS (
    SELECT k, src FROM experiment_jobs
    WHERE exp_id = $exp AND cell = $cell AND arm_label = $arm AND seed = $seed
    LIMIT 1
), changes AS (
    SELECT p.t_ms, p.node, p.n, p.stream, p.parent, p.reason FROM job JOIN parents p USING (k, src)
), times AS (
    -- Every second of the run, and $t.
    SELECT range::DOUBLE AS t FROM range(0, (SELECT max(t_ms) // 1000 + 2 FROM changes))
    UNION
    SELECT $t::DOUBLE
), edges AS (
    -- Each node's parent of each stream as each second ends: its last
    -- change by then, a pass's changes (one t_ms) in their order, n.
    SELECT ts.t, c.stream, c.node, arg_max_null(c.parent, (c.t_ms, c.n)) AS parent
    FROM times ts JOIN changes c ON c.t_ms <= ts.t * 1000 + 999
    GROUP BY ALL
), depths AS (
    SELECT t, stream, node, 1 AS depth FROM edges WHERE parent = 0
    UNION ALL
    -- A cycle that doesn't reach the seed is never reached; the bound
    -- stops one that does.
    SELECT e.t, e.stream, e.node, d.depth + 1
    FROM edges e JOIN depths d ON e.t = d.t AND e.stream = d.stream AND e.parent = d.node
    WHERE d.depth < 64
), placed AS (
    SELECT e.t, e.stream, e.node, e.parent, d.depth
    FROM edges e LEFT JOIN depths d USING (t, stream, node)
)
SELECT 'depth' AS part, t, 'mean depth' AS series, avg(depth) AS value, NULL::BIGINT AS stream,
       NULL::BIGINT AS node, NULL::BIGINT AS parent
FROM placed WHERE t = floor(t) GROUP BY t
UNION ALL
SELECT 'depth', t, 'most depth', max(depth), NULL, NULL, NULL FROM placed WHERE t = floor(t) GROUP BY t
UNION ALL
SELECT 'depth', t, 'share without a parent', avg((depth IS NULL)::INT), NULL, NULL, NULL
FROM placed WHERE t = floor(t) GROUP BY t
UNION ALL
SELECT 'change', t_ms // 1000, reason, count(*), NULL, NULL, NULL FROM changes GROUP BY ALL
UNION ALL
SELECT 'snapshot', t, NULL, depth, stream, node, parent FROM placed WHERE t = $t::DOUBLE
ORDER BY part, series, t, stream, node
