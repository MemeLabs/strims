-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Views and macros over simdb.py's views (its module docs). A statement
-- runs only when the selection has the tables its `-- needs:` line names.
-- Each keeps the views' `k` and `src`, so it joins them USING (k, src)
-- (simdb.py sql --help).

-- A run's reads in bins of `bin_ms`, per viewer: how many, and their lag.
-- needs: reads
CREATE OR REPLACE MACRO read_bins(bin_ms) AS TABLE
    SELECT k, src, viewer, t_ms // bin_ms * bin_ms AS t_ms, count(*) AS reads,
           quantile_cont(lag_ms, 0.5) AS lag_p50, quantile_cont(lag_ms, 0.99) AS lag_p99, max(lag_ms) AS lag_max
    FROM reads GROUP BY ALL;

-- Read lag pooled across a run's viewers in bins of `bin_ms`.
-- needs: reads
CREATE OR REPLACE MACRO run_read_bins(bin_ms) AS TABLE
    SELECT k, src, t_ms // bin_ms * bin_ms AS t_ms, count(*) AS reads,
           quantile_cont(lag_ms, 0.5) AS lag_p50, quantile_cont(lag_ms, 0.99) AS lag_p99, max(lag_ms) AS lag_max
    FROM reads GROUP BY ALL;

-- The fine bins regrouped into `bin_ms` (a multiple of the runs' own):
-- counters summed; a gauge's sum means nothing, read gauges from `bins`.
-- needs: bins
CREATE OR REPLACE MACRO bins_by(bin_ms) AS TABLE
    SELECT k, src, t_ms // bin_ms * bin_ms AS t_ms, sum(COLUMNS(* EXCLUDE (k, src, bin, t_ms)))
    FROM bins GROUP BY ALL;

-- Each second with its verdict.
-- needs: seconds verdict_seconds
CREATE OR REPLACE VIEW sec AS
    SELECT * FROM seconds JOIN verdict_seconds USING (k, src, sec);

-- Every event, the verdicts' (derived from the series) and, where the
-- runs wrote them, the sims' own (`events`: times in µs, with the node).
-- needs: derived_events
CREATE OR REPLACE VIEW ev AS
    SELECT k, src, sec * 1000 AS t_ms, kind, value, -1::BIGINT AS node, NULL::VARCHAR AS detail
    FROM derived_events;
-- needs: derived_events events
CREATE OR REPLACE VIEW ev AS
    SELECT k, src, sec * 1000 AS t_ms, kind, value, -1::BIGINT AS node, NULL::VARCHAR AS detail
    FROM derived_events
    UNION ALL
    SELECT k, src, t_us / 1000.0 AS t_ms, kind, value::DOUBLE, node, detail FROM events;

-- The runs of some cells and arms (empty lists for all): a farm run's
-- cell by its name or by the suite cell alone (<cell>@<run>).
-- needs: runs
CREATE OR REPLACE MACRO pick(cells, arms) AS TABLE
    SELECT * FROM runs
    WHERE (len(cells) = 0 OR list_contains(cells, cell) OR list_contains(cells, split_part(cell, '@', 1)))
      AND (len(arms) = 0 OR list_contains(arms, arm));
