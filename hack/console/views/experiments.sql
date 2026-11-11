-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Every experiment, newest first: what it asked, its arms with their
-- commits' subjects, tiers, seeds, cost, and how much of its jobs' data the
-- catalog holds (availability: a table `local`, still on S3 for `fetch`, or
-- `expired` past S3's 7 days). $search matches the id, question, tiers and
-- arms; '' for all.
WITH arms AS (
    -- An arm's commit and its subject: a --refs arm is its own commit, a
    -- --sets arm (knob_value) runs the experiment's one commit.
    SELECT e.exp_id, e.arm_label, any_value(j.commit) AS commit, count(*) AS jobs
    FROM experiment_jobs e JOIN jobs j USING (k, src)
    GROUP BY ALL
), arm_lists AS (
    SELECT a.exp_id,
           list(a.arm_label || coalesce(' ' || left(a.commit, 8), '') || coalesce(' ' || left(c.subject, 72), '')
                ORDER BY a.arm_label) AS arms
    FROM arms a LEFT JOIN commits c ON c.sha = a.commit
    GROUP BY ALL
), data AS (
    -- Each job's reads (the bulky table a timeline wants) and rows: where they are.
    SELECT e.exp_id,
           count(DISTINCT e.k) AS jobs_known,
           count(DISTINCT e.k) FILTER (WHERE a.tbl = 'runs' AND a.state = 'local') AS rows_local,
           count(DISTINCT e.k) FILTER (WHERE a.tbl = 'reads' AND a.state = 'local') AS reads_local,
           count(DISTINCT e.k) FILTER (WHERE a.tbl = 'reads' AND a.state = 's3') AS reads_s3,
           count(DISTINCT e.k) FILTER (WHERE a.tbl = 'reads' AND a.state = 'expired') AS reads_expired
    FROM experiment_jobs e LEFT JOIN availability a USING (k)
    GROUP BY ALL
)
SELECT x.exp_id, x.kind, x.started_at, x.question, l.arms, x.base, x.tiers,
       len(x.seeds) AS seeds, list_min(x.seeds) AS first_seed, x.cells, x.jobs,
       round(x.cost_usd, 4) AS cost_usd, round(x.s3_usd, 4) AS s3_usd,
       d.rows_local, d.reads_local, d.reads_s3, d.reads_expired, x.pending
FROM experiments x
LEFT JOIN arm_lists l USING (exp_id)
LEFT JOIN data d USING (exp_id)
WHERE $search = ''
   OR x.exp_id ILIKE '%' || $search || '%'
   OR x.question ILIKE '%' || $search || '%'
   OR array_to_string(x.tiers, ' ') ILIKE '%' || $search || '%'
   OR array_to_string(l.arms, ' ') ILIKE '%' || $search || '%'
ORDER BY x.started_at DESC NULLS LAST, x.exp_id
