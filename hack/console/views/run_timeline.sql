-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- One run (an experiment's cell, arm and seed) as tracks on one time axis,
-- long form: each row is one mark of one `track`, at t_s seconds from the
-- stream's start (t2_s its end, for spans).
--
--   lag      the viewers' lag per second (viewer_seconds: each viewer's
--            p99 in strims-sim, its last read's in the ppspp sim), its
--            p50, p99 and max across the viewers there
--   heat     each viewer's lag each second: a startup tail is a block at
--            the left, an intermittent spike a stripe
--   stall    each stall at a playout delay of $delay ms, $min_stall_ms or
--            longer (the ppspp sim stalls per 4 KiB chunk: many are tiny):
--            the sims' `stalls` table, or for a run without one, its
--            reads played as macros.sql's playback() plays them
--   phase    the verdict's phase each second (startup, after_down,
--            after_up, steady), and `unhealthy` seconds
--   episode  the verdict's unhealthy spells and collapses
--   seed     the seed's meter reading (capacity) and the uplink it had
--            (avail), Mbit/s, from bins
--   copies   the adaptive seed's copies
--   event    the sims' events (arrive, join, first_read, caught_up,
--            depart, parent_departed, uplink_rate, competing_*, latency,
--            ...) and the verdict's (seed_down, seed_up, onset, collapse,
--            ...)
--   bytes    what the seed and the viewers wrote each second by kind
--            of message (bins' *_bytes) but data, kbit/s
--   data     the chunk data they wrote, kbit/s
--   packets  the packets their uplinks sent each second
WITH job AS (
    SELECT k, src FROM experiment_jobs
    WHERE exp_id = $exp AND cell = $cell AND arm_label = $arm AND seed = $seed
    LIMIT 1
), vs AS (
    SELECT v.viewer, v.sec, v.lag_ms
    FROM job JOIN viewer_seconds v USING (k, src)
    WHERE v.lag_ms >= 0
), lag AS (
    SELECT sec, quantile_cont(lag_ms, 0.5) AS p50, quantile_cont(lag_ms, 0.99) AS p99, max(lag_ms) AS max
    FROM vs GROUP BY sec
), written AS (
    -- The sims' own stalls at $delay.
    SELECT s.viewer, s.t_ms, s.stall_ms FROM job JOIN stalls s USING (k, src) WHERE s.delay_ms = $delay
), firsts AS (
    -- For a run without a stalls table: playback($delay) over its reads.
    SELECT r.viewer, r.t_ms - r.lag_ms AS due, r.lag_ms
    FROM job JOIN reads r USING (k, src)
    WHERE NOT EXISTS (SELECT 1 FROM job JOIN stalls USING (k, src))
    QUALIFY row_number() OVER (PARTITION BY r.viewer, r.seq ORDER BY r.t_ms, r.lag_ms) = 1
), dues AS (
    SELECT viewer, due, max(lag_ms) AS lag_ms FROM firsts GROUP BY ALL
), played AS (
    SELECT viewer, due,
           greatest($delay, max(lag_ms) OVER (PARTITION BY viewer ORDER BY due ROWS UNBOUNDED PRECEDING)) AS offset_ms
    FROM dues
), steps AS (
    SELECT *, lag(offset_ms) OVER (PARTITION BY viewer ORDER BY due) AS before_ms FROM played
), stall AS (
    SELECT viewer, t_ms, stall_ms FROM written
    UNION ALL
    SELECT viewer, due + before_ms, offset_ms - before_ms FROM steps WHERE offset_ms > before_ms
), b AS (
    SELECT bn.* FROM job JOIN bins bn USING (k, src)
)
SELECT 'lag' AS track, sec::DOUBLE AS t_s, NULL::DOUBLE AS t2_s, series, NULL::BIGINT AS viewer,
       value::DOUBLE AS value, NULL::VARCHAR AS detail
FROM lag UNPIVOT (value FOR series IN (p50, p99, max))
UNION ALL
SELECT 'heat', sec, sec + 1, NULL, viewer, greatest(lag_ms, 1), NULL FROM vs
UNION ALL
SELECT 'stall', t_ms / 1000, (t_ms + stall_ms) / 1000, NULL, viewer, stall_ms, NULL
FROM stall WHERE stall_ms >= $min_stall_ms
UNION ALL
SELECT 'phase', sec, sec + 1, phase, NULL, NULL, NULL FROM job JOIN verdict_seconds USING (k, src)
UNION ALL
SELECT 'phase', sec, sec + 1, 'unhealthy', NULL, NULL,
       concat_ws(' ', CASE WHEN trees_bad THEN 'trees' END, CASE WHEN deliv_bad THEN 'delivery' END,
                 CASE WHEN lag_high THEN 'lag_high' END, CASE WHEN lag_rise THEN 'lag_rise' END)
FROM job JOIN verdict_seconds USING (k, src) WHERE NOT healthy
UNION ALL
SELECT 'episode', start, stop, kind, NULL, len, NULL FROM job JOIN episodes USING (k, src) WHERE kind <> 'healthy'
UNION ALL
SELECT 'seed', t_ms / 1000, NULL, series, NULL, value * 8 / 1e6, NULL
FROM b UNPIVOT (value FOR series IN (capacity, avail))
UNION ALL
SELECT 'copies', t_ms / 1000, NULL, 'copies', NULL, copies, NULL FROM b WHERE copies IS NOT NULL
UNION ALL
SELECT 'event', e.t_ms / 1000, NULL, e.kind, e.node, e.value, e.detail FROM job JOIN events e USING (k, src)
UNION ALL
SELECT 'event', d.sec, NULL, d.kind, NULL, d.value, 'verdict' FROM job JOIN derived_events d USING (k, src)
UNION ALL
SELECT CASE WHEN kind = 'data' THEN 'data' ELSE 'bytes' END, sec, NULL, kind, NULL, sum(value) * 8 / 1000, NULL
FROM (
    -- The seed's and the viewers' message bytes (*_up_* are the uplinks'
    -- wire totals, not a kind), stream and session folded into state.
    SELECT t_ms // 1000 AS sec, value,
           CASE WHEN kind IN ('stream', 'session') THEN 'state' ELSE kind END AS kind
    FROM (
        SELECT t_ms, regexp_extract(series, '^(?:seed|viewer)_(.*)_bytes$', 1) AS kind, value
        FROM (SELECT t_ms, COLUMNS('^(seed|viewer)_(.*)_bytes$') FROM b)
        UNPIVOT (value FOR series IN (COLUMNS(* EXCLUDE (t_ms))))
        WHERE series NOT LIKE '%\_up\_%' ESCAPE '\'
    )
)
GROUP BY ALL
UNION ALL
SELECT 'packets', t_ms // 1000, NULL, 'packets', NULL,
       sum(coalesce(seed_up_packets, 0) + coalesce(viewer_up_packets, 0)), NULL
FROM b GROUP BY ALL
ORDER BY track, series, t_s
