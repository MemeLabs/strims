-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- One node of a cell and arm across the seeds: how far its upload meter
-- read from the uplink it had, second by second, from meter_samples (the
-- samples the row's meter_seed_p* and meter_{pop}_p* score). The seeds
-- are those where the node is the seed or an honest viewer, as those
-- scores read them: the dishonest mix draws its hostile viewers seed by
-- seed, so a node hostile in some seeds is left out of those. Each seed
-- gives the node's last sample of each second; its error is the meter's
-- capacity ÷ what the node could upload then (1 exact, under 1 low). Long
-- form, `track` on every row, t_s to t2_s the second from the stream's
-- start:
--
--   error   the spectrogram: in each band of the error 0.05 wide (lo to
--           hi, `error` its label; the top band holds every error of 2 or
--           more), the seeds whose error was in it that second (`seeds`),
--           of the seeds with one (`seeds_read`), and their share
--   uplink  what the node could upload that second across the seeds
--           (`seeds`), Mbit/s: their p10, p50 and p90
--
-- A second before the node's meter read anything has no error, and a node
-- without an uplink (outside WAN mode, without a rate limit) none at all.
WITH runs AS (
    SELECT k, src FROM experiment_jobs
    WHERE exp_id = $exp AND cell = $cell AND arm_label = $arm
), last AS (
    -- Each run's last sample of each second.
    SELECT k, t_ms // 1000 AS sec, capacity_bps, avail_bps
    FROM runs JOIN meter_samples USING (k, src)
    WHERE node = $node AND role <> 'hostile'
    QUALIFY row_number() OVER (PARTITION BY k, src, t_ms // 1000 ORDER BY t_ms DESC) = 1
), bands AS (
    -- In integers, so a reading exactly at a band's edge starts it.
    SELECT sec, least(capacity_bps * 20 // avail_bps, 40) AS band, count(*) AS seeds
    FROM last
    WHERE capacity_bps IS NOT NULL AND avail_bps IS NOT NULL
    GROUP BY ALL
), errors AS (
    SELECT *, sum(seeds) OVER (PARTITION BY sec) AS seeds_read FROM bands
)
SELECT 'error' AS track, sec::DOUBLE AS t_s, (sec + 1)::DOUBLE AS t2_s,
       band / 20 AS lo, (band + 1) / 20 AS hi,
       CASE WHEN band = 40 THEN '≥ 2' ELSE printf('%.2f–%.2f', band / 20, (band + 1) / 20) END AS error,
       seeds, seeds_read::BIGINT AS seeds_read, seeds / seeds_read AS share,
       NULL::DOUBLE AS p10, NULL::DOUBLE AS p50, NULL::DOUBLE AS p90
FROM errors
UNION ALL
SELECT 'uplink', sec, sec + 1, NULL, NULL, NULL, count(*), NULL, NULL,
       sketch_quantile(avail_bps, 10) * 8 / 1e6, sketch_quantile(avail_bps, 50) * 8 / 1e6,
       sketch_quantile(avail_bps, 90) * 8 / 1e6
FROM last
WHERE avail_bps IS NOT NULL
GROUP BY sec
ORDER BY track, t_s, lo
