-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- suite.py's scoring as the catalog's macros: what its scorecards, arm
-- scores and races compute, here once (suite.py prints what they return).
-- catalog.py loads them (scoring_sql) after what it makes itself:
-- metric_defs, job_runs, job_metrics and job_metrics_of from suite.py's
-- METRICS, and t975, Student's t. It fills in each @{...}: a constant of
-- suite.py's, verdict.py's or its own, or a row field (r.<field>), the
-- field where the rows have it, else NULL. A macro's `-- needs:` line names
-- the tables (or table.column) it reads that a catalog indexed by older
-- code may lack: a session's own objects over such a catalog make it one
-- that fails saying so (catalog.needed). A table macro read as a fallback
-- gives instead, with `-- else none:` lines of its columns, those columns
-- and no rows.
--
-- A side is one arm's runs, a list of {'cell', 'seed', 'k', 'src'}
-- structs: the job k is that arm's run of the cell at the seed, and src
-- the source of the run's own copy (a farm index line's flush, a local
-- invocation's), whose host numbers (job_runs' wall_s, rss_mb) it reads,
-- or NULL for the job's (jobs.src); a job's row and verdict are its own
-- source's, which every copy shares. Two sides pair by (cell, seed): the
-- seeds of A in a cell that B has too (calibrate pairs one arm's seeds
-- N+1..2N with its 1..N by naming them 1..N). A side's order is its runs'
-- order: traces_of breaks ties between cells by it.
-- Only runs whose row the catalog holds count. arm_side(exp, arm) is an
-- experiment's arm as a side (experiment_jobs), and exp_cells(exp_a, exp_b)
-- the cells of exp_b that exp_a has, in exp_b's order: pairs, effects,
-- collapse_test, traces, arm_cells and arm_score take (exp_a, arm_a,
-- exp_b, arm_b) and score arm_b of exp_b against arm_a of exp_a over those
-- cells, as `suite.py score exp_a:arm_a exp_b:arm_b` does; the *_of macros
-- take a cell list and two sides.
--
--   SELECT cell, metric, effect, lo, hi, mark FROM effects('s8ed1e', 'p4tf', 's8ed1e', 'vouch')
--   WHERE mark <> ' ' ORDER BY cell_ord, ord;

-- The binary exponent of a positive a: floor(log2(a)), one less or more
-- where log2 rounds across a power of two (sketch_floor's).
CREATE OR REPLACE MACRO sketch_exp(a) AS
    floor(log2(a)) - (pow(2.0, floor(log2(a))) > a)::INT + (pow(2.0, floor(log2(a)) + 1) <= a)::INT;

-- The lower bound of the bucket of a positive a at binary exponent e:
-- 2^e (1 + m/@{sketch_sub}), m the top bits of a / 2^e - 1, every step
-- exact for a double (sketch_floor's).
CREATE OR REPLACE MACRO sketch_lo(a, e) AS
    pow(2.0, e) * (1 + floor((a / pow(2.0, e) - 1) * @{sketch_sub}) / @{sketch_sub});

-- x read through the lab's and the sims' log sketch (simlab.sketch,
-- strims_base::stats::sketch): the greatest point of its grid not above x,
-- @{sketch_sub} buckets a doubling. A positive x reads as its bucket's
-- lower bound (never above it, at most 1/129 under, integers under 256
-- exactly), 0 as itself, a negative x as its magnitude rounded up to the
-- grid, negated; NaN and the infinities as they are. A DOUBLE, an integer
-- x (to 2^53) as the integer it reads as.
CREATE OR REPLACE MACRO sketch_floor(x) AS
    CASE WHEN NOT isfinite(x) THEN x::DOUBLE
         WHEN x > 0 THEN sketch_lo(x, sketch_exp(x))
         WHEN x < 0 THEN CASE WHEN sketch_lo(-x, sketch_exp(-x)) = -x THEN x::DOUBLE
                              ELSE -(sketch_lo(-x, sketch_exp(-x)) + pow(2.0, sketch_exp(-x)) / @{sketch_sub}) END
         WHEN x = 0 THEN 0::DOUBLE END;

-- The p-th percentile (an integer from 0 to 100, a constant) of the values
-- x, read through the sketch (sketch_floor): the value of rank max(1,
-- ceil(p n / 100)) of x's n, NULLs left out, as quantile_disc gives it
-- for p / 100 as a DECIMAL, whose rank it computes in integers
-- (test_sketch holds it to the rank for every n to 1000 and p 0 to 100).
-- NULL for no values. FILTER is an aggregate's alone: over a subset,
-- sketch_quantile(CASE WHEN ... THEN x END, p).
CREATE OR REPLACE MACRO sketch_quantile(x, p) AS sketch_floor(quantile_disc(x, p * 0.01));

-- sketch_quantile of each of the percentiles ps (a constant list of
-- integers), as a list in their order.
CREATE OR REPLACE MACRO sketch_quantiles(x, ps) AS
    list_transform(quantile_disc(x, list_transform(ps, lambda sk_p: sk_p * 0.01)), lambda sk_v: sketch_floor(sk_v));

-- Student's t at 97.5 % at df_ degrees of freedom: t975's (catalog.py's
-- t975, df 1 to @{t975_df}), and past them the Cornish-Fisher expansion in
-- 1/df (Abramowitz and Stegun 26.7.5) to 1/df^4, within 1e-10 there.
CREATE OR REPLACE MACRO t975_at(df_) AS
    coalesce((SELECT t FROM t975 WHERE df = df_),
             @{t975_z} + @{t975_g1} / df_ + @{t975_g2} / pow(df_, 2) + @{t975_g3} / pow(df_, 3)
             + @{t975_g4} / pow(df_, 4));

-- The sum of the DOUBLEs xs in their order as Python's sum() adds floats
-- (3.12 on): Neumaier's compensated summation, each addition's rounding
-- error carried apart and added to the sum once at the end, where it is
-- finite and not zero. The scorecards' sums are this: effects_of's means
-- and the sums of squares under their intervals, arm_score_of's geometric
-- means and fisher_p's sum, so those are the Python scoring's bit for bit
-- (hack/sim-suite/golden.py's reference): a left fold strays from it by
-- last bits, which a value on a tie at the places a scorecard prints shows.
-- No other sum is: dynamics' means of the rows' playback fields are
-- DuckDB's avg, the other macros' sums are of integers, and a metric's own
-- SQL (metrics.toml's: depth_mean's six terms) adds as written, as the
-- reference restates it. 0 for an empty list, NULL for NULL or a NULL
-- among xs.
CREATE OR REPLACE MACRO compensated_sum(xs) AS
    list_transform([list_reduce(list_transform(xs, lambda cs_x: {'s': cs_x::DOUBLE, 'c': 0.0::DOUBLE}),
                                lambda cs_acc, cs_y: {'s': cs_acc.s + cs_y.s,
                                                      'c': cs_acc.c + CASE WHEN abs(cs_acc.s) >= abs(cs_y.s)
                                                                           THEN (cs_acc.s - (cs_acc.s + cs_y.s)) + cs_y.s
                                                                           ELSE (cs_y.s - (cs_acc.s + cs_y.s)) + cs_acc.s
                                                                           END},
                                {'s': 0.0::DOUBLE, 'c': 0.0::DOUBLE})],
                   lambda cs_acc: CASE WHEN cs_acc.c <> 0 AND isfinite(cs_acc.c) THEN cs_acc.s + cs_acc.c
                                       ELSE cs_acc.s END)[1];

-- The hypergeometric probability of x of hits_ in the first n_ of n_ + n2_.
CREATE OR REPLACE MACRO hypergeom_p(x_, n_, n2_, hits_) AS
    exp(lgamma(n_ + 1) - lgamma(x_ + 1) - lgamma(n_ - x_ + 1)
        + lgamma(n2_ + 1) - lgamma(hits_ - x_ + 1) - lgamma(n2_ - hits_ + x_ + 1)
        - lgamma(n_ + n2_ + 1) + lgamma(hits_ + 1) + lgamma(n_ + n2_ - hits_ + 1));

-- The two-sided Fisher exact p of k_ in n_ against m_ in n2_: the
-- probabilities, in order, of every split no likelier than the one seen.
CREATE OR REPLACE MACRO fisher_p(k_, n_, m_, n2_) AS
    least(1.0::DOUBLE, compensated_sum(list_filter(
        list_transform(range(greatest(0, k_ + m_ - n2_), least(n_, k_ + m_) + 1),
                       lambda x: hypergeom_p(x, n_, n2_, k_ + m_)),
        lambda pr: pr <= hypergeom_p(k_, n_, n2_, k_ + m_) * (1 + 1e-9))));

-- A paired effect's mark: + better, - worse, ~ moved where no direction is
-- better, ' ' a tie. It moved when its interval excludes no change (1 for
-- a ratio, 0 for a difference) and it passes the tie band (|ln x| past
-- ln(1 + tie), |x| past tie).
CREATE OR REPLACE MACRO effect_mark(better_, kind_, tie_, x_, lo_, hi_) AS
    CASE WHEN x_ IS NULL THEN ' '
         WHEN NOT CASE WHEN kind_ = 'ratio' THEN (lo_ > 1 OR hi_ < 1) AND abs(ln(x_)) > ln(1 + tie_)
                       ELSE (lo_ > 0 OR hi_ < 0) AND abs(x_) > tie_ END THEN ' '
         WHEN better_ IS NULL THEN '~'
         WHEN (CASE WHEN kind_ = 'ratio' THEN x_ > 1 ELSE x_ > 0 END) = (better_ = 'higher') THEN '+'
         ELSE '-' END;

-- A side's runs that the catalog holds a row of: cell, seed, k, src (the
-- job's source), run_src (the run's: src where the side names none) and
-- i, the run's place in the side.
CREATE OR REPLACE MACRO side_jobs(side) AS TABLE
    SELECT u.cell, u.seed::BIGINT AS seed, u.k, j.src, coalesce(u.src, j.src) AS run_src, u.i
    FROM (SELECT unnest(side, recursive := true), generate_subscripts(side, 1) AS i) u
    JOIN jobs j ON j.k = u.k
    SEMI JOIN rows r ON r.k = j.k AND r.src = j.src
    QUALIFY row_number() OVER (PARTITION BY u.cell, u.seed ORDER BY u.i) = 1;

-- Every metric both sides' runs at a cell and seed have a value of
-- (job_metrics_of): cell, seed, metric, a_value, b_value, and the runs'
-- jobs, a_k and b_k.
CREATE OR REPLACE MACRO pairs_of(a_jobs, b_jobs) AS TABLE
    WITH sa AS (FROM side_jobs(a_jobs)), sb AS (FROM side_jobs(b_jobs)),
    ab AS (
        SELECT sa.cell, sa.seed, sa.k AS a_k, sa.run_src AS a_src, sb.k AS b_k, sb.run_src AS b_src
        FROM sa JOIN sb ON sb.cell = sa.cell AND sb.seed = sa.seed),
    runs AS (SELECT cell, seed, a_k AS k, a_src AS src, true AS is_a FROM ab
             UNION ALL SELECT cell, seed, b_k, b_src, false FROM ab),
    vals AS (
        SELECT u.cell, u.seed, u.is_a, u.k, v.metric, v.value
        FROM runs u JOIN job_metrics_of(list_transform(a_jobs || b_jobs, lambda s: {'k': s.k, 'src': s.src})) v
          ON v.k = u.k AND v.src = u.src)
    SELECT cell, seed, metric, any_value(value) FILTER (WHERE is_a) AS a_value,
           any_value(value) FILTER (WHERE NOT is_a) AS b_value, any_value(k) FILTER (WHERE is_a) AS a_k,
           any_value(k) FILTER (WHERE NOT is_a) AS b_k
    FROM vals GROUP BY cell, seed, metric
    HAVING count(*) FILTER (WHERE is_a) > 0 AND count(*) FILTER (WHERE NOT is_a) > 0;

-- Each metric's paired effect of B against A in each cell of cell_list:
-- pairs (the seeds paired), a_median and b_median (each side's median
-- over them, sketch_quantile's p50), n (the pairs it is over: a ratio's
-- those with both values above 0), effect (the geometric mean ratio, exp
-- of the mean ln(b/a), or the mean difference b - a, over the seeds in
-- order), lo and hi (its 95 % t-interval; one pair gives 0 or -inf to inf),
-- their sums compensated_sum's, mark (effect_mark; h for a host metric that
-- moved) and metric_defs' columns.
CREATE OR REPLACE MACRO effects_of(cell_list, a_jobs, b_jobs) AS TABLE
    WITH p AS (
        SELECT p.*, m.ord, m.kind, m.better, m.tie, m.host, m.shown,
               CASE WHEN m.kind = 'ratio' THEN CASE WHEN p.a_value > 0 AND p.b_value > 0
                                                    THEN ln(p.b_value / p.a_value) END
                    ELSE p.b_value - p.a_value END AS x
        FROM pairs_of(a_jobs, b_jobs) p JOIN metric_defs m ON m.name = p.metric
        WHERE list_contains(cell_list, p.cell)),
    g AS (
        SELECT cell, metric, any_value(ord) AS ord, any_value(kind) AS kind, any_value(better) AS better,
               any_value(tie) AS tie, any_value(host) AS host, any_value(shown) AS shown, count(*) AS pairs,
               sketch_quantile(a_value, 50) AS a_median, sketch_quantile(b_value, 50) AS b_median,
               list(x ORDER BY seed) FILTER (WHERE x IS NOT NULL) AS xs
        FROM p GROUP BY cell, metric),
    s AS (SELECT *, len(xs) AS n, compensated_sum(xs) / len(xs) AS mean FROM g),
    h AS (
        SELECT *, CASE WHEN n >= 2 THEN t975_at(n - 1)
                       * sqrt(compensated_sum(list_transform(xs, lambda y: (y - mean) ** 2)) / (n - 1)) / sqrt(n)
                  END AS half
        FROM s),
    e AS (
        SELECT *,
               CASE WHEN kind = 'ratio' THEN exp(mean) ELSE mean END AS effect,
               CASE WHEN n IS NULL THEN NULL
                    WHEN n < 2 THEN CASE WHEN kind = 'ratio' THEN 0.0::DOUBLE ELSE '-inf'::DOUBLE END
                    WHEN kind = 'ratio' THEN exp(mean - half) ELSE mean - half END AS lo,
               CASE WHEN n IS NULL THEN NULL WHEN n < 2 THEN 'inf'::DOUBLE
                    WHEN kind = 'ratio' THEN exp(mean + half) ELSE mean + half END AS hi
        FROM h)
    SELECT cell, list_position(cell_list, cell) AS cell_ord, metric, ord, kind, better, tie, host, shown, pairs,
           a_median, b_median, n, effect, lo, hi,
           CASE WHEN host AND effect_mark(better, kind, tie, effect, lo, hi) <> ' ' THEN 'h'
                ELSE effect_mark(better, kind, tie, effect, lo, hi) END AS mark
    FROM e;

-- Each cell of cell_list's collapsed runs (verdicts.collapsed) over the
-- seeds both sides ran: ka of A's, kb of B's, n seeds, p their two-sided
-- Fisher exact p, mark + (fewer, p < 0.05), - (more), ' ' (p >= 0.05) or
-- NULL where neither collapsed.
CREATE OR REPLACE MACRO collapse_test_of(cell_list, a_jobs, b_jobs) AS TABLE
    WITH c AS (
        SELECT sa.cell, count(*) AS n, count(*) FILTER (WHERE va.collapsed) AS ka,
               count(*) FILTER (WHERE vb.collapsed) AS kb
        FROM side_jobs(a_jobs) sa JOIN side_jobs(b_jobs) sb ON sb.cell = sa.cell AND sb.seed = sa.seed
        LEFT JOIN verdicts va ON va.k = sa.k AND va.src = sa.src
        LEFT JOIN verdicts vb ON vb.k = sb.k AND vb.src = sb.src
        WHERE list_contains(cell_list, sa.cell) GROUP BY sa.cell)
    SELECT cell, list_position(cell_list, cell) AS cell_ord, ka, kb, n, fisher_p(ka, n, kb, n) AS p,
           CASE WHEN ka = 0 AND kb = 0 THEN NULL WHEN fisher_p(ka, n, kb, n) >= 0.05 THEN ' '
                WHEN kb < ka THEN '+' ELSE '-' END AS mark
    FROM c;

-- How many of each cell's runs both sides have, with a trace each, B ran
-- as A did (runs, same): the same trace, and the same reads where both
-- carry their digest (jobs.reads_hash). Every cell the sides share, not
-- only a cell list's; a_first is the cell's first place in A.
CREATE OR REPLACE MACRO traces_of(a_jobs, b_jobs) AS TABLE
    SELECT sa.cell, min(sa.i) AS a_first, count(*) AS runs,
           count(*) FILTER (WHERE ja.trace = jb.trace AND (coalesce(ja.reads_hash, '') = ''
                                                          OR coalesce(jb.reads_hash, '') = ''
                                                          OR ja.reads_hash = jb.reads_hash)) AS same
    FROM side_jobs(a_jobs) sa JOIN side_jobs(b_jobs) sb ON sb.cell = sa.cell AND sb.seed = sa.seed
    JOIN jobs ja ON ja.k = sa.k JOIN jobs jb ON jb.k = sb.k
    WHERE coalesce(ja.trace, '') <> '' AND coalesce(jb.trace, '') <> ''
    GROUP BY sa.cell;

-- An arm (B) against the base (A) cell by cell, as races and farm runs
-- rank arms: each cell of cell_list both ran, its collapses (ka, kb, n, p),
-- its lag p99 ratio (lag, lag_lo, lag_hi) and wire per useful byte ratio
-- (wire), and the guardrails it breaks there (breaks, in order):
--   collapsed    more collapsed runs (kb > ka) with p < 0.05;
--   lag_ceiling  ranked for overhead (overhead true), the base's median
--                lag p99 (a, sketch_quantile's p50) under lag_ceiling and
--                the arm's (b) not;
--   worse        a metric_defs guard 'worse' metric (delivery) marked -;
--   guard        in a cell the base never collapsed, a guard 'lo' metric's
--                ratio interval lying past guard_ratio (past lag_budget
--                when ranked for overhead: infinite, none, by default);
--   below_one    a guard 'hi' metric's (defector_gain) ratio interval
--                lying under 1: defecting pays more.
-- A break: {cell, kind, metric, x (the effect; lag_ceiling's ceiling), a, b,
-- ka, kb, n}.
CREATE OR REPLACE MACRO arm_cells_of(cell_list, a_jobs, b_jobs, overhead := false, lag_ceiling := @{lag_ceiling},
                                     lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    WITH c AS (FROM collapse_test_of(cell_list, a_jobs, b_jobs)),
    e AS (
        SELECT e.*, m.guard, m.guard_ord FROM effects_of(cell_list, a_jobs, b_jobs) e
        JOIN metric_defs m ON m.name = e.metric
        WHERE m.guard IS NOT NULL OR e.metric IN ('lag_p99_ms', 'wire/useful')),
    mid AS (
        SELECT cell, sketch_quantile(a_value, 50) AS a_mid, sketch_quantile(b_value, 50) AS b_mid
        FROM pairs_of(a_jobs, b_jobs) WHERE metric = 'lag_p99_ms' AND list_contains(cell_list, cell) GROUP BY cell),
    brk AS (
        SELECT cell, 0 AS o, 0 AS mo, {'cell': cell, 'kind': 'collapsed', 'metric': 'collapsed', 'x': p, 'a': NULL::DOUBLE,
               'b': NULL::DOUBLE, 'ka': ka, 'kb': kb, 'n': n} AS br
        FROM c WHERE kb > ka AND p < 0.05
        UNION ALL
        SELECT cell, 1, 0, {'cell': cell, 'kind': 'lag_ceiling', 'metric': 'lag_p99_ms', 'x': lag_ceiling::DOUBLE, 'a': a_mid,
               'b': b_mid, 'ka': NULL::BIGINT, 'kb': NULL::BIGINT, 'n': NULL::BIGINT}
        FROM mid WHERE overhead AND a_mid < lag_ceiling AND lag_ceiling <= b_mid
        UNION ALL
        SELECT cell, 2, guard_ord, {'cell': cell, 'kind': 'worse', 'metric': metric, 'x': effect, 'a': NULL, 'b': NULL,
               'ka': NULL, 'kb': NULL, 'n': NULL}
        FROM e WHERE guard = 'worse' AND mark = '-'
        UNION ALL
        SELECT e.cell, 3, e.guard_ord, {'cell': e.cell, 'kind': 'guard', 'metric': e.metric, 'x': e.effect, 'a': NULL, 'b': NULL,
               'ka': NULL, 'kb': NULL, 'n': NULL}
        FROM e JOIN c USING (cell)
        WHERE e.guard = 'lo' AND c.ka = 0 AND e.effect IS NOT NULL
          AND e.lo > CASE WHEN overhead THEN lag_budget ELSE guard_ratio END
        UNION ALL
        SELECT cell, 4, guard_ord, {'cell': cell, 'kind': 'below_one', 'metric': metric, 'x': effect, 'a': NULL, 'b': NULL,
               'ka': NULL, 'kb': NULL, 'n': NULL}
        FROM e WHERE guard = 'hi' AND effect IS NOT NULL AND hi < 1),
    brks AS (SELECT cell, list(br ORDER BY o, mo) AS breaks FROM brk GROUP BY cell)
    SELECT c.cell, c.cell_ord, c.ka, c.kb, c.n, c.p, l.effect AS lag, l.lo AS lag_lo, l.hi AS lag_hi,
           w.effect AS wire, coalesce(b.breaks, []) AS breaks
    FROM c LEFT JOIN e l ON l.cell = c.cell AND l.metric = 'lag_p99_ms'
    LEFT JOIN e w ON w.cell = c.cell AND w.metric = 'wire/useful'
    LEFT JOIN brks b ON b.cell = c.cell;

-- An arm's score against the base over the cells (arm_cells_of): its
-- collapsed runs and the base's, the geometric means over cells of its
-- lag p99 ratio (p99) and its wire per useful byte ratio (wire), every
-- break, and rank_key, by which arms rank: broken ones last, then fewer
-- collapsed runs, then the lower p99 (wire when ranked for overhead).
CREATE OR REPLACE MACRO arm_score_of(cell_list, a_jobs, b_jobs, overhead := false, lag_ceiling := @{lag_ceiling},
                                     lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    WITH c AS (FROM arm_cells_of(cell_list, a_jobs, b_jobs, overhead := overhead, lag_ceiling := lag_ceiling,
                                 lag_budget := lag_budget, guard_ratio := guard_ratio)),
    t AS (
        SELECT coalesce(sum(kb), 0) AS collapsed, coalesce(sum(ka), 0) AS base_collapsed,
               exp(compensated_sum(list(ln(lag) ORDER BY cell_ord) FILTER (WHERE lag IS NOT NULL)) / count(lag)) AS p99,
               exp(compensated_sum(list(ln(wire) ORDER BY cell_ord) FILTER (WHERE wire IS NOT NULL)) / count(wire))
                   AS wire,
               coalesce(flatten(list(breaks ORDER BY cell_ord)), []) AS breaks
        FROM c)
    SELECT collapsed, base_collapsed, p99, wire, breaks, len(breaks) > 0 AS broken, overhead::BOOLEAN AS overhead,
           {'broken': len(breaks) > 0, 'collapsed': collapsed,
            'by': coalesce(CASE WHEN overhead THEN wire ELSE p99 END, 'inf'::DOUBLE)} AS rank_key
    FROM t;

-- An experiment's arm as a side: its jobs (experiment_jobs), each run's
-- source its index line's flush, in its cells' order. An arm holding two
-- jobs at one cell and seed is an error.
CREATE OR REPLACE MACRO arm_side(exp_name, arm_name) AS (
    SELECT CASE WHEN count(DISTINCT {'cell': cell, 'seed': seed}) < count(*)
                THEN error('experiment ' || exp_name || '''s arm ' || arm_name || ' holds two jobs at one cell and seed')
                ELSE list({'cell': cell, 'seed': seed, 'k': k, 'src': flush} ORDER BY matrix, cell, seed) END
    FROM (SELECT DISTINCT cell, seed, k, flush, matrix FROM experiment_jobs
          WHERE exp_id = exp_name AND arm_label = arm_name));

-- The cells of exp_b that exp_a has too, in exp_b's order (its matrices').
CREATE OR REPLACE MACRO exp_cells(exp_a, exp_b) AS (
    SELECT coalesce(list(cell ORDER BY matrix, cell), []) FROM experiment_cells
    WHERE exp_id = exp_b AND cell IN (SELECT cell FROM experiment_cells WHERE exp_id = exp_a));

-- pairs_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO pairs(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM pairs_of(arm_side(exp_a, arm_a), arm_side(exp_b, arm_b)) WHERE list_contains(exp_cells(exp_a, exp_b), cell);
-- effects_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells: a scorecard's lines.
CREATE OR REPLACE MACRO effects(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM effects_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b));
-- collapse_test_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO collapse_test(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM collapse_test_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b));
-- traces_of over arm_b of exp_b against arm_a of exp_a, every cell they share.
CREATE OR REPLACE MACRO traces(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM traces_of(arm_side(exp_a, arm_a), arm_side(exp_b, arm_b));
-- arm_cells_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO arm_cells(exp_a, arm_a, exp_b, arm_b, overhead := false, lag_ceiling := @{lag_ceiling},
                                  lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    FROM arm_cells_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b), overhead := overhead,
                      lag_ceiling := lag_ceiling, lag_budget := lag_budget, guard_ratio := guard_ratio);
-- arm_score_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO arm_score(exp_a, arm_a, exp_b, arm_b, overhead := false, lag_ceiling := @{lag_ceiling},
                                  lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    FROM arm_score_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b), overhead := overhead,
                      lag_ceiling := lag_ceiling, lag_budget := lag_budget, guard_ratio := guard_ratio);

-- The cells where every arm of experiment exp_name collapsed on every
-- seed it ran (verdicts.collapsed): no arm can be told from another there.
CREATE OR REPLACE MACRO structural(exp_name) AS TABLE
    SELECT e.cell, count(DISTINCT e.arm_label) AS arms, count(*) AS runs
    FROM experiment_jobs e JOIN verdicts v ON v.k = e.k AND v.src = e.src
    WHERE e.exp_id = exp_name
    GROUP BY e.cell HAVING bool_and(v.collapsed)
    ORDER BY min(e.matrix), e.cell;

-- Each viewer's phases in the runs `runs`, a list of structs with a run's
-- `k` and `src` (its tables' source, experiment_jobs' or jobs'; other
-- fields left alone), as spans of its seconds start <= sec < stop: k,
-- src, viewer, phase, start, stop. A viewer's second, and a read or a
-- stall in it, is in the phase of the span holding it: over the
-- verdict's seconds, viewer_phases' (the verdict's phase rule, its own
-- joins' startups among it); from the verdict's last second on (the end
-- of verdict_seconds and of the viewers' spans; a run the verdict didn't
-- judge from its start), @{tail}, the run's tail, every viewer's
-- (viewers). A second of the verdict's that none of its viewer's spans
-- holds (the viewer wasn't there, by the verdict's timeline) is in no
-- phase. A query joins its rows to the spans, its counts and what it
-- divides them by alike.
-- needs: viewer_phases verdict_seconds viewers
CREATE OR REPLACE MACRO viewer_phases_of(runs) AS TABLE
    WITH j_ AS (SELECT DISTINCT k, src FROM (SELECT unnest(runs, recursive := true))),
    spans_ AS (SELECT x.k, x.src, x.viewer, x.phase, x.start, x.stop FROM viewer_phases x SEMI JOIN j_ USING (k, src)),
    end_ AS (
        SELECT j_.k, j_.src, coalesce(max(e.stop), 0) AS stop
        FROM j_ LEFT JOIN (SELECT k, src, stop FROM spans_ UNION ALL
                           SELECT k, src, sec + 1 FROM verdict_seconds SEMI JOIN j_ USING (k, src)) e USING (k, src)
        GROUP BY j_.k, j_.src)
    SELECT * FROM spans_
    UNION ALL
    SELECT w.k, w.src, w.viewer, @{tail}, end_.stop, 9223372036854775807::BIGINT
    FROM viewers w JOIN end_ USING (k, src);

-- Each viewer's stalls in the runs `runs`, a list of structs with a run's
-- `k` and `src` (its tables' source; other fields left alone), at a
-- playout delay of `delay_ms`, played from their reads as the sims'
-- `stalls` tables have them (strims-wan's `playout`): a unit read twice
-- counts at its first read, the units due at one time (t_ms - lag_ms)
-- play together, as late as the latest of them, and in due order each
-- plays at due + offset, the offset the most of `delay_ms` and the lags
-- so far. Every rise of the offset past the first unit's is a stall: it
-- starts at the unit's play time at the old offset (t_ms, the reads'
-- clock) and lasts the rise (stall_ms): k, src, viewer, delay_ms, t_ms,
-- stall_ms. macros.sql's playback(delay_ms) is it over every run of
-- simdb.py's views.
-- needs: reads
CREATE OR REPLACE MACRO playback_of(runs, delay_ms) AS TABLE
    WITH j_ AS (SELECT DISTINCT k, src FROM (SELECT unnest(runs, recursive := true))),
    firsts_ AS (
        SELECT r.k, r.src, r.viewer, r.t_ms - r.lag_ms AS due, r.lag_ms FROM reads r SEMI JOIN j_ USING (k, src)
        QUALIFY row_number() OVER (PARTITION BY r.k, r.src, r.viewer, r.seq ORDER BY r.t_ms, r.lag_ms) = 1),
    dues_ AS (SELECT k, src, viewer, due, max(lag_ms) AS lag_ms FROM firsts_ GROUP BY k, src, viewer, due),
    played_ AS (
        SELECT k, src, viewer, due,
               greatest(delay_ms, max(lag_ms) OVER (PARTITION BY k, src, viewer ORDER BY due
                                                    ROWS UNBOUNDED PRECEDING)) AS offset_ms
        FROM dues_),
    steps_ AS (SELECT *, lag(offset_ms) OVER (PARTITION BY k, src, viewer ORDER BY due) AS before_ms FROM played_)
    SELECT k, src, viewer, delay_ms::BIGINT AS delay_ms, due + before_ms AS t_ms, offset_ms - before_ms AS stall_ms
    FROM steps_ WHERE offset_ms > before_ms;

-- The honest viewers' playout stalls at a playout delay of `delay_ms` in
-- the runs `runs`, a list of structs with a run's `k` and `src` (other
-- fields left alone), by the phase of the viewer's second they fall in
-- (viewer_phases_of): the seconds the viewers were there and reading
-- (viewer_seconds with a lag, viewer_s), the stalls of the sims' `stalls`
-- tables that began in them (stalls) and how long each lasted (durations,
-- ms; stalled_ms their sum): k, src, phase, viewer_s, stalls, stalled_ms,
-- durations. A phase with stalls and none of the run's seconds holds
-- viewer_s 0. Dishonest viewers (viewers.hostile) are left out: the
-- experience metrics count honest viewers. Stalls per viewer-minute are
-- stalls over viewer_s / 60, the stalled share stalled_ms over
-- viewer_s * 1000.
-- needs: stalls viewer_seconds viewers viewer_phases verdict_seconds
CREATE OR REPLACE MACRO stall_rate_of(runs, delay_ms) AS TABLE
    WITH j_ AS (SELECT DISTINCT k, src FROM (SELECT unnest(runs, recursive := true))),
    at_ AS (SELECT delay_ms::BIGINT AS d),
    honest_ AS (SELECT w.k, w.src, w.viewer FROM viewers w SEMI JOIN j_ USING (k, src) WHERE NOT w.hostile),
    spans_ AS (SELECT p.k, p.src, p.viewer, p.phase, p.start, p.stop FROM viewer_phases_of(runs) p
               SEMI JOIN honest_ USING (k, src, viewer)),
    watched_ AS (
        SELECT p.k, p.src, p.phase, count(*) AS viewer_s
        FROM viewer_seconds v JOIN spans_ p ON p.k = v.k AND p.src = v.src AND p.viewer = v.viewer
                                           AND v.sec >= p.start AND v.sec < p.stop
        WHERE v.lag_ms >= 0
        GROUP BY ALL),
    stalled_ AS (
        SELECT p.k, p.src, p.phase, count(*) AS stalls, sum(s.stall_ms) AS stalled_ms,
               list(s.stall_ms ORDER BY s.stall_ms) AS durations
        FROM stalls s JOIN spans_ p ON p.k = s.k AND p.src = s.src AND p.viewer = s.viewer
                                    AND s.t_ms // 1000 >= p.start AND s.t_ms // 1000 < p.stop
        WHERE s.delay_ms = (SELECT d FROM at_)
        GROUP BY ALL)
    SELECT k, src, phase, coalesce(w.viewer_s, 0) AS viewer_s, coalesce(st.stalls, 0) AS stalls,
           coalesce(st.stalled_ms, 0) AS stalled_ms, coalesce(st.durations, []) AS durations
    FROM watched_ w FULL JOIN stalled_ st USING (k, src, phase);

-- The honest viewers' lag each second of the runs `jobs`, a list of
-- structs with a job's `k` (its tables its own source's, jobs.src; other
-- fields left alone, so a side serves): k, src, viewer, sec, lag_ms,
-- frames and arrive_ms (NULL for a viewer there from the start). lag_ms is
-- viewer_seconds', carried from the viewer's last second with a read
-- +1000 ms a second without one, as verdict.py's lags() does: NULL before
-- its first read.
CREATE OR REPLACE MACRO viewer_lag_of(jobs) AS TABLE
    WITH j AS (
        SELECT DISTINCT b.k, b.src FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k),
    vs AS (
        SELECT s.k, s.src, s.viewer, s.sec, s.lag_ms, s.frames, w.arrive_ms
        FROM viewer_seconds s SEMI JOIN j USING (k, src)
        JOIN viewers w USING (k, src, viewer) WHERE NOT w.hostile)
    SELECT k, src, viewer, sec,
           coalesce(lag_ms, last_value(lag_ms IGNORE NULLS) OVER w
                    + 1000 * (sec - last_value(CASE WHEN lag_ms IS NOT NULL THEN sec END IGNORE NULLS) OVER w))
               AS lag_ms,
           frames, arrive_ms
    FROM vs WINDOW w AS (PARTITION BY k, src, viewer ORDER BY sec ROWS UNBOUNDED PRECEDING);

-- Each arm and cell of experiment exp_name, in the terms that tell a
-- startup tail from a steady spike:
--   runs, collapsed          its runs and the collapsed ones;
--   lag_p99_<phase>_ms       the p99 of the honest viewers' lag over their
--                            seconds of startup and of steady, and of
--                            every after_* phase pooled (after: an uplink
--                            step, a departure, a disruption), over the
--                            runs: viewer_lag_of's carried lag. A
--                            second's phase is the viewer's
--                            (viewer_phases_of: the verdict's rule, its
--                            own joins' startups among it), over the
--                            verdict's seconds; dynamics_phases gives
--                            each phase's apart;
--   spike_s_<D>              per run, the verdict's seconds past startup,
--                            the run's and its own, in which an honest
--                            viewer lags past D ms: a player D behind
--                            would stall then;
--   stalls_per_min_<D>,      the honest viewers' playback at delay D, the
--   stalled_share_<D>,       rows' <name>_honest (the mean over runs; a
--   startup_ms_<D>           row from before the sim split its viewers'
--                            playback by population, which ran no
--                            dishonest viewers, its <name>, the same
--                            there; NULL in rows from before the sims
--                            played them);
--   unhealthy_max_s,         the longest unhealthy spell (episodes) of any
--   unhealthy_p50_s          run, and the median of each run's longest;
-- for D in @{delay_target_ms} and @{delay_max_ms} (verdict.py's
-- DELIVERY_TARGET_MS and DELIVERY_MAX_MS).
-- needs: viewer_phases verdict_seconds viewers
CREATE OR REPLACE MACRO dynamics(exp_name) AS TABLE
    WITH ej AS (
        SELECT DISTINCT cell, arm_label, k, src, matrix FROM experiment_jobs WHERE exp_id = exp_name),
    lags AS (
        FROM viewer_lag_of((SELECT list({'k': k}) FROM experiment_jobs WHERE exp_id = exp_name))),
    phased AS (
        SELECT c.*, p.phase
        FROM lags c
        JOIN viewer_phases_of((SELECT list({'k': k, 'src': src}) FROM experiment_jobs WHERE exp_id = exp_name)) p
          ON p.k = c.k AND p.src = c.src AND p.viewer = c.viewer AND c.sec >= p.start AND c.sec < p.stop
        WHERE p.phase <> @{tail}),
    lagq AS (
        SELECT e.cell, e.arm_label,
               sketch_quantile(CASE WHEN p.phase = 'startup' THEN p.lag_ms END, 99) AS lag_p99_startup_ms,
               sketch_quantile(CASE WHEN p.phase = 'steady' THEN p.lag_ms END, 99) AS lag_p99_steady_ms,
               sketch_quantile(CASE WHEN starts_with(p.phase, 'after_') THEN p.lag_ms END, 99) AS lag_p99_after_ms
        FROM phased p JOIN ej e USING (k, src) GROUP BY ALL),
    spikes AS (
        SELECT k, src, count(DISTINCT sec) FILTER (WHERE lag_ms > @{delay_target_ms}) AS s1,
               count(DISTINCT sec) FILTER (WHERE lag_ms > @{delay_max_ms}) AS s2
        FROM phased WHERE phase <> 'startup' GROUP BY ALL),
    spells AS (
        SELECT k, src, max(len) AS longest FROM episodes WHERE kind = 'unhealthy' GROUP BY ALL),
    runs AS (
        SELECT e.cell, e.arm_label, any_value(e.matrix) AS matrix, count(*) AS runs,
               count(*) FILTER (WHERE v.collapsed) AS collapsed,
               sum(coalesce(sp.s1, 0)) / count(*) AS spike_s_@{delay_target_ms},
               sum(coalesce(sp.s2, 0)) / count(*) AS spike_s_@{delay_max_ms},
               avg(coalesce(@{r.stalls_per_min_@{delay_target_ms}_honest}, @{r.stalls_per_min_@{delay_target_ms}}))
                   AS stalls_per_min_@{delay_target_ms},
               avg(coalesce(@{r.stalls_per_min_@{delay_max_ms}_honest}, @{r.stalls_per_min_@{delay_max_ms}}))
                   AS stalls_per_min_@{delay_max_ms},
               avg(coalesce(@{r.stalled_share_@{delay_target_ms}_honest}, @{r.stalled_share_@{delay_target_ms}}))
                   AS stalled_share_@{delay_target_ms},
               avg(coalesce(@{r.stalled_share_@{delay_max_ms}_honest}, @{r.stalled_share_@{delay_max_ms}}))
                   AS stalled_share_@{delay_max_ms},
               avg(coalesce(@{r.startup_ms_@{delay_target_ms}_honest}, @{r.startup_ms_@{delay_target_ms}}))
                   AS startup_ms_@{delay_target_ms},
               avg(coalesce(@{r.startup_ms_@{delay_max_ms}_honest}, @{r.startup_ms_@{delay_max_ms}}))
                   AS startup_ms_@{delay_max_ms},
               coalesce(max(u.longest), 0) AS unhealthy_max_s,
               sketch_quantile(coalesce(u.longest, 0), 50) AS unhealthy_p50_s
        FROM ej e JOIN verdicts v USING (k, src) JOIN rows r USING (k, src)
        LEFT JOIN spikes sp USING (k, src) LEFT JOIN spells u USING (k, src)
        GROUP BY ALL)
    SELECT r.cell, r.arm_label, r.runs, r.collapsed, l.* EXCLUDE (cell, arm_label), r.* EXCLUDE (cell, arm_label,
           matrix, runs, collapsed)
    FROM runs r LEFT JOIN lagq l USING (cell, arm_label)
    ORDER BY r.matrix, r.cell, r.arm_label;

-- Each arm and cell of experiment exp_name by phase, long form: cell,
-- arm_label, phase (the vocabulary's, in its order), viewer_s (the honest
-- viewers' seconds of the phase with a lag, over the runs), share (of the
-- cell and arm's), and the lag's lag_p50_ms, lag_p90_ms, lag_p99_ms and
-- lag_max_ms over them: viewer_lag_of's carried lag over the verdict's
-- seconds, each in its viewer's phase (viewer_phases_of), as dynamics
-- reads them.
-- needs: viewer_phases verdict_seconds viewers
CREATE OR REPLACE MACRO dynamics_phases(exp_name) AS TABLE
    WITH ej AS (
        SELECT DISTINCT cell, arm_label, k, src, matrix FROM experiment_jobs WHERE exp_id = exp_name),
    lags AS (
        FROM viewer_lag_of((SELECT list({'k': k}) FROM experiment_jobs WHERE exp_id = exp_name))),
    phased AS (
        SELECT c.*, p.phase
        FROM lags c
        JOIN viewer_phases_of((SELECT list({'k': k, 'src': src}) FROM experiment_jobs WHERE exp_id = exp_name)) p
          ON p.k = c.k AND p.src = c.src AND p.viewer = c.viewer AND c.sec >= p.start AND c.sec < p.stop
        WHERE c.lag_ms IS NOT NULL AND p.phase <> @{tail}),
    q AS (
        SELECT e.cell, e.arm_label, any_value(e.matrix) AS matrix, p.phase, count(*) AS viewer_s,
               sketch_quantile(p.lag_ms, 50) AS lag_p50_ms, sketch_quantile(p.lag_ms, 90) AS lag_p90_ms,
               sketch_quantile(p.lag_ms, 99) AS lag_p99_ms, max(p.lag_ms) AS lag_max_ms
        FROM phased p JOIN ej e USING (k, src) GROUP BY e.cell, e.arm_label, p.phase)
    SELECT cell, arm_label, phase, viewer_s, viewer_s / sum(viewer_s) OVER (PARTITION BY cell, arm_label) AS share,
           lag_p50_ms, lag_p90_ms, lag_p99_ms, lag_max_ms
    FROM q
    ORDER BY matrix, cell, arm_label, list_position(@{phases}, phase), phase;

-- The honest viewers' lag (viewer_lag_of) against the seconds since each
-- phase window of experiment exp_name's runs (phase_windows: the
-- verdict's, the phase rule's) opened, and since each span's end, from
-- `before` seconds before to `after` after, by cell, arm, kind, phase
-- and since: cell, arm_label, kind, phase, since, rel_s, windows,
-- viewer_s and lag_p50_ms, lag_p90_ms, lag_p99_ms. kind is the window's
-- anchor's (`start` the stream's start), phase its phase (a level's by
-- its direction); since is 'anchor' for the seconds since its anchor,
-- 'end' for those since its span's end (a span with an end alone): the
-- curves its horizon_s is read from. A viewer's second sec is rel_s =
-- floor(sec + 0.5 - t) after t, as the verdict places a second in a
-- window. A window of the run's takes every honest viewer's seconds of
-- its run, a join's (phase_windows.viewer, the viewer's own) only its
-- viewer's. A run's windows of one kind and phase at one time (of one
-- viewer, for a join) are one, and a second within two windows of a kind
-- counts in each. windows is how many windows hold a viewer-second at
-- rel_s, and viewer_s how many viewer-seconds with a lag they hold there
-- (the quantiles').
-- needs: phase_windows
CREATE OR REPLACE MACRO lag_since(exp_name, before := 10, after := 120) AS TABLE
    WITH ej AS (
        SELECT DISTINCT cell, arm_label, k, src, matrix FROM experiment_jobs WHERE exp_id = exp_name),
    anchors AS (
        SELECT row_number() OVER () AS w, * FROM (
            SELECT DISTINCT p.k, p.src, p.kind, p.phase, a.since, p.viewer IS NOT NULL AS own,
                   CASE a.since WHEN 'anchor' THEN p.anchor_s ELSE p.end_s END AS t, p.viewer
            FROM phase_windows p SEMI JOIN ej USING (k, src)
            JOIN (VALUES ('anchor'), ('end')) a(since) ON a.since = 'anchor' OR p.end_s IS NOT NULL)),
    secs AS (
        SELECT w, k, src, kind, phase, since, own, viewer, t,
               unnest(range(ceil(t - before - 0.5)::BIGINT, ceil(t + after - 0.5)::BIGINT)) AS sec
        FROM anchors),
    lags AS (
        FROM viewer_lag_of((SELECT list({'k': k}) FROM experiment_jobs WHERE exp_id = exp_name))),
    hits AS (
        SELECT s.w, s.k, s.src, s.kind, s.phase, s.since, floor(s.sec + 0.5 - s.t)::BIGINT AS rel_s, l.lag_ms
        FROM secs s JOIN lags l USING (k, src, sec) WHERE NOT s.own
        UNION ALL
        SELECT s.w, s.k, s.src, s.kind, s.phase, s.since, floor(s.sec + 0.5 - s.t)::BIGINT AS rel_s, l.lag_ms
        FROM secs s JOIN lags l USING (k, src, viewer, sec) WHERE s.own)
    SELECT e.cell, e.arm_label, h.kind, h.phase, h.since, h.rel_s, count(DISTINCT h.w) AS windows,
           count(h.lag_ms) AS viewer_s, sketch_quantile(h.lag_ms, 50) AS lag_p50_ms,
           sketch_quantile(h.lag_ms, 90) AS lag_p90_ms, sketch_quantile(h.lag_ms, 99) AS lag_p99_ms
    FROM hits h JOIN ej e USING (k, src)
    GROUP BY e.cell, e.arm_label, h.kind, h.phase, h.since, h.rel_s
    ORDER BY min(e.matrix), e.cell, e.arm_label, h.kind, h.phase, h.since, h.rel_s;

-- Seed coverage (simlab.coverage, whose docs say what each definition
-- gives and why), over the runs `jobs`, a list of structs with a job's
-- `k` (its tables its own source's, jobs.src). Each `*_of(jobs)` macro is
-- its `*_in(runs)` body over coverage_runs_of's rows, which a caller that
-- holds them passes once (`runs`, a list of them) rather than deriving
-- them again for each. Their CTEs are named cv_*, so a caller's own CTE
-- isn't read in their place.
--
-- The facts of each run, a row a run: k, src, seed_node and seed_from
-- (the seed the sim recorded, series_meta's `seed`: `series_meta`; else
-- the node that published the most, arrivals' `own` rows: `arrivals`),
-- streams and streams_from (the stream count the sim recorded,
-- series_meta's `streams` or the row's `streams`:
-- `series_meta`, `row`; else inferred, one more than the greatest stream
-- its parents name, `parents`, else its asks', `asks`, else one, NULL;
-- a chunk's stream is its seq mod streams), interval_ms (the run's
-- publication interval: the median gap between a stream's consecutive
-- publications, a publication being the stream's chunks at one ms of the
-- seed's own rows, one or several), end_ms (the end of its last second in
-- `seconds`, or its last hold if that is later: strims-sim runs on past
-- its stream), arrivals_rows and parents_rows (the run's rows of each the
-- catalog holds: without arrivals a run has no chunks, without parents
-- its I1, I2 and its late chunks' classes and causes are NULL), and
-- seed_decisions_rows and seed_sends_rows (its seed log's rows,
-- coverage_seed_logs_of's, 0 where the catalog has none: a run with
-- neither has no seed log, and its I2, I3 and late chunks' causes are
-- NULL).
-- needs: arrivals parents
CREATE OR REPLACE MACRO coverage_runs_of(jobs) AS TABLE
    WITH cv_job_keys AS (
        SELECT DISTINCT b.k, b.src FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k),
    cv_meta AS (
        SELECT u.k, u.src, max(TRY_CAST(json_extract(u.meta, '$.seed') AS BIGINT)) AS seed,
               max(TRY_CAST(json_extract(u.meta, '$.streams') AS BIGINT)) AS streams
        FROM raw.runs u SEMI JOIN cv_job_keys USING (k, src) GROUP BY ALL),
    cv_own AS MATERIALIZED (
        SELECT x.k, x.src, x.node, x.seq, x.t_ms FROM arrivals x SEMI JOIN cv_job_keys USING (k, src)
        WHERE x.how = 'own'),
    cv_published AS (
        SELECT k, src, arg_max(node, (n, -node)) AS node
        FROM (SELECT k, src, node, count(*) AS n FROM cv_own GROUP BY ALL) GROUP BY ALL),
    cv_seed AS (
        SELECT j.k, j.src, coalesce(m.seed, p.node) AS node,
               CASE WHEN m.seed IS NOT NULL THEN 'series_meta' WHEN p.node IS NOT NULL THEN 'arrivals' END
                   AS seed_from
        FROM cv_job_keys j LEFT JOIN cv_meta m USING (k, src) LEFT JOIN cv_published p USING (k, src)),
    cv_row_streams AS (
        SELECT r.k, r.src, max(@{r.streams})::BIGINT AS n FROM rows r SEMI JOIN cv_job_keys USING (k, src)
        GROUP BY ALL HAVING max(@{r.streams}) IS NOT NULL),
    cv_parent_streams AS (
        SELECT k, src, max(stream) + 1 AS n, count(*) AS rows_n FROM parents SEMI JOIN cv_job_keys USING (k, src)
        GROUP BY ALL),
    -- Only a run whose count is neither recorded nor its push parents'
    -- reads its asks.
    cv_unknown AS (
        SELECT j.k FROM cv_job_keys j LEFT JOIN cv_meta m USING (k, src) LEFT JOIN cv_row_streams w USING (k, src)
        LEFT JOIN cv_parent_streams p USING (k, src) WHERE m.streams IS NULL AND w.n IS NULL AND p.n IS NULL),
    cv_ask_streams AS (
        FROM coverage_ask_streams_of((SELECT list({'k': k}) FROM cv_unknown))),
    cv_streams AS (
        SELECT j.k, j.src, coalesce(m.streams, w.n, p.n, a.n, 1) AS streams,
               CASE WHEN m.streams IS NOT NULL THEN 'series_meta' WHEN w.n IS NOT NULL THEN 'row'
                    WHEN p.n IS NOT NULL THEN 'parents' WHEN a.n IS NOT NULL THEN 'asks' END AS streams_from
        FROM cv_job_keys j LEFT JOIN cv_meta m USING (k, src) LEFT JOIN cv_row_streams w USING (k, src)
        LEFT JOIN cv_parent_streams p USING (k, src) LEFT JOIN cv_ask_streams a USING (k, src)),
    cv_pub AS (
        SELECT x.k, x.src, x.seq % t.streams AS stream, x.seq, min(x.t_ms) AS pub_ms
        FROM cv_own x JOIN cv_seed s ON s.k = x.k AND s.src = x.src AND s.node = x.node
        JOIN cv_streams t ON t.k = x.k AND t.src = x.src GROUP BY ALL),
    cv_gaps AS (
        SELECT k, src, sketch_quantile(gap, 50) AS interval_ms FROM (
            SELECT k, src, pub_ms - lag(pub_ms) OVER (PARTITION BY k, src, stream ORDER BY pub_ms) AS gap
            FROM (SELECT DISTINCT k, src, stream, pub_ms FROM cv_pub))
        WHERE gap IS NOT NULL GROUP BY ALL),
    cv_secs AS (
        SELECT k, src, (max(sec) + 1) * 1000 AS end_ms FROM seconds SEMI JOIN cv_job_keys USING (k, src)
        GROUP BY ALL),
    cv_holds AS (
        SELECT k, src, max(t_ms) AS end_ms, count(*) AS rows_n FROM arrivals SEMI JOIN cv_job_keys USING (k, src)
        GROUP BY ALL),
    cv_logs AS (
        FROM coverage_seed_logs_of(jobs))
    SELECT j.k, j.src, s.node AS seed_node, s.seed_from, t.streams, t.streams_from, g.interval_ms,
           greatest(e.end_ms, h.end_ms) AS end_ms, coalesce(h.rows_n, 0) AS arrivals_rows,
           coalesce(p.rows_n, 0) AS parents_rows, coalesce(l.decisions, 0) AS seed_decisions_rows,
           coalesce(l.sends, 0) AS seed_sends_rows
    FROM cv_job_keys j LEFT JOIN cv_seed s USING (k, src) LEFT JOIN cv_streams t USING (k, src)
    LEFT JOIN cv_gaps g USING (k, src) LEFT JOIN cv_secs e USING (k, src) LEFT JOIN cv_holds h USING (k, src)
    LEFT JOIN cv_parent_streams p USING (k, src) LEFT JOIN cv_logs l USING (k, src);

-- coverage_runs_of's last fallback for a run's stream count: one more
-- than the greatest stream the asks of the runs `jobs` name, a row a run
-- (k, src, n) that has asks. raw.asks, the rows the asks view gives of a
-- job without its truth (a join of every arrivals row). A catalog
-- without asks has no rows of it, rather than a macro that fails.
-- needs: asks
-- else none: k VARCHAR, src VARCHAR, n BIGINT
CREATE OR REPLACE MACRO coverage_ask_streams_of(jobs) AS TABLE
    SELECT a.k, a.src, max(a.stream) + 1 AS n FROM raw.asks a
    SEMI JOIN (SELECT DISTINCT b.k, b.src FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k)
        USING (k, src)
    GROUP BY ALL;

-- coverage_runs_of's count of each run's seed log, a row a run of the runs
-- `jobs` that has one: k, src, decisions and sends (its rows of
-- raw.seed_decisions and raw.seed_sends, the tables as the sim wrote
-- them; the seed_sends view adds a join of every arrivals row). A catalog
-- without the seed log's tables has no rows of it, rather than a macro
-- that fails.
-- needs: seed_decisions seed_sends
-- else none: k VARCHAR, src VARCHAR, decisions BIGINT, sends BIGINT
CREATE OR REPLACE MACRO coverage_seed_logs_of(jobs) AS TABLE
    WITH cv_job_keys AS (
        SELECT DISTINCT b.k, b.src FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k),
    cv_d AS (
        SELECT k, src, count(*) AS n FROM raw.seed_decisions SEMI JOIN cv_job_keys USING (k, src) GROUP BY ALL),
    cv_s AS (
        SELECT k, src, count(*) AS n FROM raw.seed_sends SEMI JOIN cv_job_keys USING (k, src) GROUP BY ALL)
    SELECT k, src, coalesce(d.n, 0) AS decisions, coalesce(s.n, 0) AS sends FROM cv_d d FULL JOIN cv_s s USING (k, src);

-- Every node's subscription to the seed in the runs `runs`
-- (coverage_runs_of's rows), from parents: k, src, node, stream, open_ms
-- (a row naming the seed the node's parent of the stream), close_ms (the
-- node's next row of the stream, else its run's end), end_reason (that
-- row's reason, NULL to the run's end) and dishonest (the node is a
-- dishonest viewer: the viewers row of its host, viewers.node, is
-- hostile; a node no viewers row names, the seed or a relay, is none). A
-- node's rows of a stream at one ms are ordered as they can have
-- happened: a @{reason_switch} first, an @{reason_open} after the ends if
-- the node had a parent before that ms (an open needs none) and before
-- them if not. A subscription of no length is none.
-- needs: parents
CREATE OR REPLACE MACRO seed_subscriptions_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
    cv_dishonest AS (
        SELECT DISTINCT v.k, v.src, v.node FROM viewers v SEMI JOIN cv_r USING (k, src) WHERE v.hostile),
    cv_p AS (
        SELECT p.k, p.src, p.t_ms, p.node, p.stream, p.parent, p.reason,
               CASE WHEN p.reason = @{reason_switch} THEN 0 WHEN p.parent IS NULL THEN 1 ELSE 2 END AS pre
        FROM parents p SEMI JOIN cv_r USING (k, src)),
    cv_before AS (
        SELECT *, lag(parent) OVER (PARTITION BY k, src, node, stream ORDER BY t_ms, pre, reason, parent)
                      AS lag_parent,
               row_number() OVER (PARTITION BY k, src, node, stream, t_ms ORDER BY pre, reason, parent) AS i
        FROM cv_p),
    cv_tied AS (
        SELECT *, first_value(lag_parent) OVER (PARTITION BY k, src, node, stream, t_ms ORDER BY i) AS before
        FROM cv_before),
    cv_ordered AS (
        SELECT k, src, t_ms, node, stream, parent, reason,
               row_number() OVER (PARTITION BY k, src, node, stream ORDER BY t_ms,
                                  CASE WHEN reason = @{reason_switch} OR (reason = @{reason_open} AND before IS NULL)
                                       THEN 0 WHEN parent IS NULL THEN 1 ELSE 2 END, reason, parent) AS ord
        FROM cv_tied),
    cv_spans AS (
        SELECT o.k, o.src, o.node, o.stream, o.parent, r.seed_node, o.t_ms AS open_ms,
               coalesce(lead(o.t_ms) OVER w, greatest(r.end_ms, o.t_ms)) AS close_ms,
               lead(o.reason) OVER w AS end_reason
        FROM cv_ordered o JOIN cv_r r USING (k, src)
        WINDOW w AS (PARTITION BY o.k, o.src, o.node, o.stream ORDER BY o.ord))
    SELECT s.k, s.src, s.node, s.stream, s.open_ms, s.close_ms, s.end_reason, d.node IS NOT NULL AS dishonest
    FROM cv_spans s LEFT JOIN cv_dishonest d ON d.k = s.k AND d.src = s.src AND d.node = s.node
    WHERE s.parent = s.seed_node AND s.close_ms > s.open_ms;

-- The seed's child subscriptions in the runs `runs` (coverage_runs_of's
-- rows): seed_subscriptions_in's of every node but the dishonest viewers
-- (the honest viewers, and a node no viewer, a relay): k, src, node,
-- stream, open_ms, close_ms and end_reason.
-- needs: parents
CREATE OR REPLACE MACRO seed_children_in(runs) AS TABLE
    SELECT k, src, node, stream, open_ms, close_ms, end_reason FROM seed_subscriptions_in(runs) WHERE NOT dishonest;

-- seed_children_in over the runs `jobs`.
-- needs: arrivals parents
CREATE OR REPLACE MACRO seed_children_of(jobs) AS TABLE
    FROM seed_children_in((SELECT list(cv_run) FROM coverage_runs_of(jobs) cv_run));

-- Each chunk the seed of a run of `runs` (coverage_runs_of's rows)
-- published, a row a chunk: k, src, seq, stream, pub_ms (the seed's own
-- arrivals row), phase (the run's, verdict_seconds', of its publication
-- second; @{tail} past the verdict's seconds), first_ms, first_node
-- and latency_ms (first_ms - pub_ms) of its earliest hold by a node but
-- the seed and the dishonest viewers (the hosts of the viewers rows that
-- are hostile, viewers.node and viewers.hostile; ties by node, any
-- incarnation), route (that hold's `push` or `answer`, else `other`, or
-- `never`: no such node held it), source (the hold's peer: `seed`;
-- `dishonest`, a dishonest viewer, which held it first; else `peer`, or
-- `unnamed` naming none, an anomaly; NULL for never), interval_ms (its
-- run's), judged (published at least @{delay_max_ms} ms, the latency
-- budget's most, before its run's end: every other chunk, held or not,
-- is censored), late (never held, or held more than its run's interval
-- (one publication interval of its stream) after its publication; NULL
-- for every chunk of a run whose interval is unknown, each stream
-- published at one ms: lateness is against the interval, so its chunks
-- are neither late nor on time, a never-held one `never` all the same)
-- and, for a judged late chunk of a run with parents rows (NULL for the
-- others, every chunk of a run of no interval among them), the seed's
-- children of its stream at its publication
-- (seed_children_in's, no dishonest viewer): children (how many),
-- dishonest_children (how many dishonest viewers were subscribed to the
-- seed on its stream then, seed_subscriptions_in's),
-- class (the first that holds: no_seed_child; released, a child's hold
-- names a release to pull, arrivals' released_ms; pushed_late, a child
-- holds it by the seed's push; child_never, no child ever held it;
-- other) and after (for no_seed_child, how the stream's last seed child
-- before it ended, end_reason's; NULL for a stream that never had one).
-- needs: arrivals arrivals.released_ms parents
CREATE OR REPLACE MACRO coverage_chunks_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
    cv_dishonest AS (
        SELECT DISTINCT v.k, v.src, v.node FROM viewers v SEMI JOIN cv_r USING (k, src) WHERE v.hostile),
    cv_arr AS MATERIALIZED (
        SELECT x.k, x.src, x.node, x.seq, x.t_ms, x.how, x.peer, x.released_ms, x.node = r.seed_node AS seed,
               d.node IS NOT NULL AS dishonest
        FROM arrivals x JOIN cv_r r USING (k, src)
        LEFT JOIN cv_dishonest d ON d.k = x.k AND d.src = x.src AND d.node = x.node),
    cv_pub AS (
        SELECT x.k, x.src, x.seq, x.seq % r.streams AS stream, min(x.t_ms) AS pub_ms
        FROM cv_arr x JOIN cv_r r USING (k, src) WHERE x.seed AND x.how = 'own' GROUP BY ALL),
    cv_first AS (
        SELECT k, src, seq, arg_min({'t_ms': t_ms, 'how': how, 'peer': peer, 'node': node}, (t_ms, node)) AS h
        FROM cv_arr WHERE NOT seed AND NOT dishonest GROUP BY ALL),
    cv_chunks_of AS (
        SELECT p.k, p.src, p.seq, p.stream, p.pub_ms, coalesce(v.phase, @{tail}) AS phase,
               f.h.t_ms AS first_ms, f.h.node AS first_node, f.h.t_ms - p.pub_ms AS latency_ms,
               CASE WHEN f.h IS NULL THEN 'never' WHEN f.h.how IN ('push', 'answer') THEN f.h.how
                    ELSE 'other' END AS route,
               CASE WHEN f.h IS NULL THEN NULL WHEN f.h.peer = r.seed_node THEN 'seed'
                    WHEN dp.node IS NOT NULL THEN 'dishonest' WHEN f.h.peer >= 0 THEN 'peer' ELSE 'unnamed' END
                   AS source,
               r.interval_ms, p.pub_ms + @{delay_max_ms} <= r.end_ms AS judged,
               CASE WHEN r.interval_ms IS NULL THEN NULL WHEN f.h IS NULL THEN true
                    ELSE f.h.t_ms - p.pub_ms > r.interval_ms END AS late,
               r.parents_rows > 0 AS parented
        FROM cv_pub p JOIN cv_r r USING (k, src) LEFT JOIN cv_first f USING (k, src, seq)
        LEFT JOIN cv_dishonest dp ON dp.k = p.k AND dp.src = p.src AND dp.node = f.h.peer
        LEFT JOIN verdict_seconds v ON v.k = p.k AND v.src = p.src AND v.sec = p.pub_ms // 1000),
    cv_subs AS (FROM seed_children_in(runs)),
    cv_dishonest_subs AS (FROM seed_subscriptions_in(runs) WHERE dishonest),
    cv_kids AS (
        SELECT c.k, c.src, c.seq, s.node FROM cv_chunks_of c
        JOIN cv_subs s ON s.k = c.k AND s.src = c.src AND s.stream = c.stream AND s.open_ms <= c.pub_ms
                       AND c.pub_ms < s.close_ms
        WHERE c.judged AND c.late IS TRUE),
    cv_kid_holds AS (
        SELECT x.k, x.src, x.node, x.seq,
               arg_min({'how': x.how, 'peer': x.peer, 'released_ms': x.released_ms}, x.t_ms) AS h
        FROM cv_arr x JOIN cv_kids USING (k, src, node, seq) GROUP BY ALL),
    cv_kid AS (
        SELECT d.k, d.src, d.seq, count(*) AS children, bool_or(h.h.released_ms IS NOT NULL) AS released,
               bool_or(h.h.how = 'push' AND h.h.peer = r.seed_node) AS pushed, bool_and(h.h IS NULL) AS never
        FROM cv_kids d LEFT JOIN cv_kid_holds h USING (k, src, node, seq) JOIN cv_r r ON r.k = d.k AND r.src = d.src
        GROUP BY ALL),
    cv_dishonest_kids AS (
        SELECT c.k, c.src, c.seq, count(*) AS n FROM cv_chunks_of c
        JOIN cv_dishonest_subs s ON s.k = c.k AND s.src = c.src AND s.stream = c.stream AND s.open_ms <= c.pub_ms
                                 AND c.pub_ms < s.close_ms
        WHERE c.judged AND c.late IS TRUE GROUP BY ALL),
    cv_ended AS (
        SELECT k, src, stream, close_ms, end_reason FROM cv_subs WHERE end_reason IS NOT NULL)
    SELECT c.* EXCLUDE (parented, classed),
           CASE WHEN c.classed THEN coalesce(d.children, 0) END AS children,
           CASE WHEN c.classed THEN coalesce(b.n, 0) END AS dishonest_children,
           CASE WHEN NOT c.classed THEN NULL WHEN d.children IS NULL THEN 'no_seed_child'
                WHEN d.released THEN 'released' WHEN d.pushed THEN 'pushed_late'
                WHEN d.never THEN 'child_never' ELSE 'other' END AS class,
           CASE WHEN c.classed AND d.children IS NULL THEN e.end_reason END AS after
    FROM (SELECT *, coalesce(judged AND late AND parented, false) AS classed FROM cv_chunks_of) c
    LEFT JOIN cv_kid d USING (k, src, seq)
    LEFT JOIN cv_dishonest_kids b ON b.k = c.k AND b.src = c.src AND b.seq = c.seq
    ASOF LEFT JOIN cv_ended e ON e.k = c.k AND e.src = c.src AND e.stream = c.stream AND c.pub_ms >= e.close_ms;

-- coverage_chunks_in over the runs `jobs`.
-- needs: arrivals arrivals.released_ms parents
CREATE OR REPLACE MACRO coverage_chunks_of(jobs) AS TABLE
    FROM coverage_chunks_in((SELECT list(cv_run) FROM coverage_runs_of(jobs) cv_run));

-- I1, the invariant that every stream a viewer watches has a seed child,
-- in the runs `runs` (coverage_runs_of's rows): each stream-second an
-- honest viewer watched (a viewer_phases second of one; every viewer
-- wants every stream) within the verdict's seconds: k, src, stream, sec,
-- phase (the run's), covered_ms (the time in it with a seed child,
-- seed_children_in's: no dishonest viewer, ms exact; 1000 less it is
-- uncovered) and child_ms (the seed's children's time in it, summed);
-- both NULL for a run with no parents rows (no push trees: no seed
-- children to count).
-- needs: parents viewer_phases
CREATE OR REPLACE MACRO seed_cover_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
    cv_subs AS (FROM seed_children_in(runs)),
    cv_ev AS (
        SELECT k, src, stream, open_ms AS t, 1 AS d FROM cv_subs
        UNION ALL SELECT k, src, stream, close_ms, -1 FROM cv_subs),
    cv_net AS (
        SELECT k, src, stream, t, sum(d) AS d FROM cv_ev GROUP BY ALL),
    -- The seed children of a stream: c from a to b.
    cv_seg AS (
        SELECT k, src, stream, t AS a, lead(t) OVER w AS b, sum(d) OVER (w ROWS UNBOUNDED PRECEDING) AS c
        FROM cv_net WINDOW w AS (PARTITION BY k, src, stream ORDER BY t)),
    cv_secs AS (
        SELECT k, src, stream, a, b, c, unnest(range(a // 1000, (b - 1) // 1000 + 1)) AS sec
        FROM cv_seg WHERE b > a AND c > 0),
    cv_covered AS (
        SELECT k, src, stream, sec, sum(least(b, (sec + 1) * 1000) - greatest(a, sec * 1000)) AS covered_ms,
               sum(c * (least(b, (sec + 1) * 1000) - greatest(a, sec * 1000))) AS child_ms
        FROM cv_secs GROUP BY ALL),
    cv_watch AS (
        SELECT DISTINCT k, src, sec FROM (
            SELECT p.k, p.src, unnest(range(p.start, p.stop)) AS sec
            FROM viewer_phases p SEMI JOIN cv_r USING (k, src)
            JOIN viewers v ON v.k = p.k AND v.src = p.src AND v.viewer = p.viewer
            WHERE NOT coalesce(v.hostile, false))),
    cv_watched AS (
        SELECT w.k, w.src, unnest(range(r.streams)) AS stream, w.sec, v.phase, r.parents_rows > 0 AS parented
        FROM cv_watch w JOIN cv_r r USING (k, src)
        JOIN verdict_seconds v ON v.k = w.k AND v.src = w.src AND v.sec = w.sec)
    SELECT s.k, s.src, s.stream, s.sec, s.phase,
           CASE WHEN s.parented THEN coalesce(c.covered_ms, 0) END AS covered_ms,
           CASE WHEN s.parented THEN coalesce(c.child_ms, 0) END AS child_ms
    FROM cv_watched s LEFT JOIN cv_covered c USING (k, src, stream, sec);

-- seed_cover_in over the runs `jobs`.
-- needs: arrivals parents viewer_phases
CREATE OR REPLACE MACRO seed_cover_of(jobs) AS TABLE
    FROM seed_cover_in((SELECT list(cv_run) FROM coverage_runs_of(jobs) cv_run));

-- I2, the invariant that the seed writes every chunk it publishes to a
-- holder of its stream unless that holder cancelled it, with the sends
-- behind a late chunk's cause (coverage_causes_in), in the runs `runs`
-- (coverage_runs_of's rows): each judged chunk (coverage_chunks_in's) of
-- a run with parents rows and a seed log, a row a chunk: k, src, seq,
-- stream, pub_ms, phase, first_ms, interval_ms, late and
-- dishonest_children (coverage_chunks_in's), children (the seed's
-- children of its stream at its publication, seed_children_in's: no
-- dishonest viewer; every judged chunk's) and, of the seed's sends of the
-- chunk to those children (seed_sends of node the run's seed, peer a
-- child, seq: pushes and answers, any incarnation; raw.seed_sends, the
-- sends as written, as the view's join of every arrivals row is slow
-- here), sends (how many), written (those of fate 'written'), cancelled
-- (those of fate 'cancelled': a push dropped at write time for the
-- child's CANCEL), fates (each distinct how and fate, as 'push stale',
-- sorted; empty where none was queued) and cancel_ms (the earliest
-- cancel_ms of a cancelled one); of the first written push (the earliest
-- written, ties by peer; NULL without one), push_ms (its t_ms),
-- push_wait_ms (its wait in the seed's queue, t_ms - queued_ms) and
-- push_held_ms (its child's hold, the seed_sends view's held_ms: the
-- child's earliest arrivals row of the chunk in the push's
-- peer_incarnation; NULL for none); unwritten (I2 broken: a chunk with
-- children none of whose sends to them was written or cancelled; NULL for
-- a chunk with no child, I1's). A send is a push or an answer here: a
-- chunk whose push went stale or was cancelled and whose child was
-- answered keeps I2, where coverage_causes_in's not_written, push-only,
-- counts it.
-- needs: arrivals arrivals.released_ms arrivals.incarnation parents seed_decisions seed_sends
-- else none: k VARCHAR, src VARCHAR, seq BIGINT, stream BIGINT, pub_ms BIGINT, phase VARCHAR, first_ms BIGINT
-- else none: interval_ms DOUBLE, late BOOLEAN, dishonest_children BIGINT, children BIGINT, sends BIGINT
-- else none: written BIGINT, cancelled BIGINT, fates VARCHAR[], cancel_ms BIGINT, push_ms BIGINT
-- else none: push_wait_ms BIGINT, push_held_ms BIGINT, unwritten BOOLEAN
CREATE OR REPLACE MACRO seed_writes_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
    cv_c AS MATERIALIZED (
        SELECT c.k, c.src, c.seq, c.stream, c.pub_ms, c.phase, c.first_ms, c.interval_ms, c.late,
               c.dishonest_children
        FROM coverage_chunks_in(runs) c JOIN cv_r r USING (k, src)
        WHERE c.judged AND r.parents_rows > 0 AND r.seed_decisions_rows + r.seed_sends_rows > 0),
    cv_kids AS (
        SELECT c.k, c.src, c.seq, s.node FROM cv_c c
        JOIN seed_children_in(runs) s ON s.k = c.k AND s.src = c.src AND s.stream = c.stream
                                      AND s.open_ms <= c.pub_ms AND c.pub_ms < s.close_ms),
    cv_kid_n AS (
        SELECT k, src, seq, count(*) AS children FROM cv_kids GROUP BY ALL),
    cv_sent AS (
        SELECT d.k, d.src, d.seq, d.node, x.peer_incarnation, x.how, x.fate, x.t_ms, x.queued_ms, x.cancel_ms
        FROM cv_kids d JOIN cv_r r USING (k, src)
        JOIN raw.seed_sends x ON x.k = d.k AND x.src = d.src AND x.node = r.seed_node AND x.peer = d.node
                              AND x.seq = d.seq),
    cv_sent_n AS (
        SELECT k, src, seq, count(*) AS sends, count(*) FILTER (WHERE fate = 'written') AS written,
               count(*) FILTER (WHERE fate = 'cancelled') AS cancelled,
               list_sort(list(DISTINCT how || ' ' || fate)) AS fates,
               min(cancel_ms) FILTER (WHERE fate = 'cancelled') AS cancel_ms,
               arg_min({'t_ms': t_ms, 'wait_ms': t_ms - queued_ms, 'node': node, 'incarnation': peer_incarnation},
                       (t_ms, node)) FILTER (WHERE how = 'push' AND fate = 'written') AS p
        FROM cv_sent GROUP BY ALL),
    -- The first written push's delivery: its child's earliest hold of the
    -- chunk in its incarnation then, as the seed_sends view's held_ms.
    cv_push_held AS (
        SELECT s.k, s.src, s.seq, min(x.t_ms) AS held_ms FROM cv_sent_n s
        JOIN arrivals x ON x.k = s.k AND x.src = s.src AND x.node = s.p.node AND x.seq = s.seq
                        AND x.incarnation = s.p.incarnation
        GROUP BY ALL)
    SELECT c.*, coalesce(n.children, 0) AS children, coalesce(s.sends, 0) AS sends,
           coalesce(s.written, 0) AS written, coalesce(s.cancelled, 0) AS cancelled,
           coalesce(s.fates, []::VARCHAR[]) AS fates, s.cancel_ms, s.p.t_ms AS push_ms,
           s.p.wait_ms AS push_wait_ms, h.held_ms AS push_held_ms,
           CASE WHEN n.children > 0 THEN coalesce(s.written, 0) = 0 AND coalesce(s.cancelled, 0) = 0 END
               AS unwritten
    FROM cv_c c LEFT JOIN cv_kid_n n USING (k, src, seq) LEFT JOIN cv_sent_n s USING (k, src, seq)
    LEFT JOIN cv_push_held h USING (k, src, seq);

-- seed_writes_in over the runs `jobs`.
-- needs: arrivals arrivals.released_ms arrivals.incarnation parents seed_decisions seed_sends
CREATE OR REPLACE MACRO seed_writes_of(jobs) AS TABLE
    FROM seed_writes_in((SELECT list(cv_run) FROM coverage_runs_of(jobs) cv_run));

-- I3, the invariant that the seed never declines a request for a stream
-- that has no subscriber, in the runs `runs` (coverage_runs_of's rows):
-- each request the run's seed declined while the stream had none (its
-- seed_decisions of kind 'request', granted false, subscribers 0: the
-- stream's subscribers at the seed before its answer, the dishonest
-- viewers' among them), a row a decision: k, src, t_ms, peer, stream, why
-- (its rule), phase (the run's, verdict_seconds', of its second; @{tail}
-- past the verdict's seconds) and dishonest (the peer is a dishonest
-- viewer: the viewers row of its host, viewers.node, is hostile).
-- needs: seed_decisions
-- else none: k VARCHAR, src VARCHAR, t_ms BIGINT, peer BIGINT, stream BIGINT, why VARCHAR, phase VARCHAR
-- else none: dishonest BOOLEAN
CREATE OR REPLACE MACRO seed_declines_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
    cv_dishonest AS (
        SELECT DISTINCT v.k, v.src, v.node FROM viewers v SEMI JOIN cv_r USING (k, src) WHERE v.hostile)
    SELECT d.k, d.src, d.t_ms, d.peer, d.stream, d.why, coalesce(v.phase, @{tail}) AS phase,
           h.node IS NOT NULL AS dishonest
    FROM seed_decisions d JOIN cv_r r ON r.k = d.k AND r.src = d.src AND r.seed_node = d.node
    LEFT JOIN cv_dishonest h ON h.k = d.k AND h.src = d.src AND h.node = d.peer
    LEFT JOIN verdict_seconds v ON v.k = d.k AND v.src = d.src AND v.sec = d.t_ms // 1000
    WHERE d.kind = 'request' AND NOT d.granted AND d.subscribers = 0;

-- seed_declines_in over the runs `jobs`.
-- needs: arrivals parents seed_decisions
CREATE OR REPLACE MACRO seed_declines_of(jobs) AS TABLE
    FROM seed_declines_in((SELECT list(cv_run) FROM coverage_runs_of(jobs) cv_run));

-- Why each late chunk was late, in the runs `runs` (coverage_runs_of's
-- rows): each judged late chunk of a run with parents rows and a seed log
-- (seed_writes_in's; a run whose interval is unknown has none, its chunks'
-- late NULL), a row a chunk: k, src, seq, stream, pub_ms, phase,
-- children and dishonest_children (seed_writes_in's), matches (every cause
-- of the tree below that holds, in its order; ['other'] for none) and
-- cause (the first, matches[1]):
--   uncovered       no seed child of its stream at its publication, nor a
--                   dishonest viewer subscribed to the seed on it
--   defectors_only  no seed child, but a dishonest viewer subscribed
--   not_written     children, but no push of it to them written (its
--                   first written push NULL), whatever was answered:
--                   I2's unwritten (seed_writes_in's) reads every send,
--                   so a chunk answered to a child keeps I2 and counts
--                   here
--   written_late    the first written push to a child waited in the
--                   seed's queue (push_wait_ms) past the run's interval
--   delivered_late  that push was written within the interval, but its
--                   child held it (push_held_ms) later than the interval
--                   after the publication
--   departed        every child departed (an events row of kind
--                   @{kind_depart} or @{kind_crash} naming it, at or after
--                   its subscription opened) by the chunk's due time (its
--                   publication plus the run's interval, when it became
--                   late) and before its first hold of the chunk, or
--                   without one: a child there at the due time without
--                   the chunk wasn't kept from it by leaving
-- and of an uncovered chunk (NULL for the others): last_end (the reason
-- the stream's last subscription to the seed, of any node, a dishonest
-- viewer's too, ended at or before its publication,
-- seed_subscriptions_in's, the least node's of those ending at one ms;
-- NULL for a stream that had none), seed_end and seed_why (the seed log's
-- removal of that subscriber within that subscription, after its open and
-- by its end: the latest of its `closed` rows of the stream and its
-- `holder_left` and `barred` rows, of those at one ms closed, then
-- holder_left, then barred, its kind and why; NULL for none) and declined
-- (the seed declined a request for the stream since that end, or since
-- the run's start, by its publication: an I3 decline, seed_declines_in's,
-- of a peer no dishonest viewer);
-- fates (seed_writes_in's) and held_at_cancel (for a chunk with a
-- cancelled send to a child: a node but the seed and the dishonest
-- viewers held it by the earliest cancel_ms, its first_ms); and of its
-- first written push (NULL without one) push_ms, push_wait_ms,
-- delivery_ms (push_held_ms - push_ms) and, over the seed's sends
-- written in the second before push_ms (raw.seed_sends: the sends as
-- written, to every peer), answer_bytes and push_bytes (their bytes,
-- answers' and pushes') and pushes (the written pushes).
-- needs: arrivals arrivals.released_ms arrivals.incarnation parents seed_decisions seed_sends events
-- else none: k VARCHAR, src VARCHAR, seq BIGINT, stream BIGINT, pub_ms BIGINT, phase VARCHAR, children BIGINT
-- else none: dishonest_children BIGINT, matches VARCHAR[], cause VARCHAR, last_end VARCHAR, seed_end VARCHAR
-- else none: seed_why VARCHAR, declined BOOLEAN, fates VARCHAR[], held_at_cancel BOOLEAN, push_ms BIGINT
-- else none: push_wait_ms BIGINT, delivery_ms BIGINT, answer_bytes BIGINT, push_bytes BIGINT, pushes BIGINT
CREATE OR REPLACE MACRO coverage_causes_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
    cv_w AS MATERIALIZED (
        FROM seed_writes_in(runs) WHERE late),
    -- A stream's subscription ends at one ms: the least node's.
    cv_ends AS (
        SELECT k, src, stream, close_ms, arg_min({'node': node, 'open_ms': open_ms, 'reason': end_reason}, node) AS e
        FROM seed_subscriptions_in(runs) WHERE end_reason IS NOT NULL GROUP BY ALL),
    cv_bare AS (
        SELECT w.k, w.src, w.seq, w.stream, w.pub_ms, e.close_ms, e.e
        FROM (FROM cv_w WHERE children = 0 AND dishonest_children = 0) w
        ASOF LEFT JOIN cv_ends e ON e.k = w.k AND e.src = w.src AND e.stream = w.stream AND w.pub_ms >= e.close_ms),
    cv_removed AS (
        SELECT b.k, b.src, b.seq,
               arg_max({'kind': d.kind, 'why': d.why},
                       (d.t_ms, CASE d.kind WHEN 'closed' THEN 2 WHEN 'holder_left' THEN 1 ELSE 0 END,
                        coalesce(d.why, ''))) AS d
        FROM cv_bare b JOIN cv_r r USING (k, src)
        JOIN seed_decisions d ON d.k = b.k AND d.src = b.src AND d.node = r.seed_node AND d.peer = b.e.node
                              AND d.t_ms > b.e.open_ms AND d.t_ms <= b.close_ms
                              AND (d.kind = 'closed' AND d.stream = b.stream OR d.kind IN ('holder_left', 'barred'))
        GROUP BY ALL),
    cv_declined AS (
        SELECT DISTINCT b.k, b.src, b.seq FROM cv_bare b
        JOIN seed_declines_in(runs) d ON d.k = b.k AND d.src = b.src AND d.stream = b.stream AND NOT d.dishonest
                                      AND (b.close_ms IS NULL OR d.t_ms >= b.close_ms) AND d.t_ms <= b.pub_ms),
    cv_kids AS (
        SELECT w.k, w.src, w.seq, s.node, s.open_ms, w.pub_ms + w.interval_ms AS due_ms FROM cv_w w
        JOIN seed_children_in(runs) s ON s.k = w.k AND s.src = w.src AND s.stream = w.stream
                                      AND s.open_ms <= w.pub_ms AND w.pub_ms < s.close_ms),
    cv_kid_holds AS (
        SELECT x.k, x.src, x.node, x.seq, min(x.t_ms) AS t_ms FROM arrivals x JOIN cv_kids USING (k, src, node, seq)
        GROUP BY ALL),
    cv_left AS (
        SELECT d.k, d.src, d.seq, d.node, count(e.t_ms) > 0 AS gone
        FROM cv_kids d LEFT JOIN cv_kid_holds h USING (k, src, node, seq)
        LEFT JOIN events e ON e.k = d.k AND e.src = d.src AND e.node = d.node
                           AND e.kind IN (@{kind_depart}, @{kind_crash}) AND e.t_ms >= d.open_ms
                           AND e.t_ms <= d.due_ms AND (h.t_ms IS NULL OR e.t_ms < h.t_ms)
        GROUP BY ALL),
    cv_departed AS (
        SELECT k, src, seq, bool_and(gone) AS departed FROM cv_left GROUP BY ALL),
    cv_load AS (
        SELECT w.k, w.src, w.seq, sum(x.bytes) FILTER (WHERE x.how = 'answer')::BIGINT AS answer_bytes,
               sum(x.bytes) FILTER (WHERE x.how = 'push')::BIGINT AS push_bytes,
               count(*) FILTER (WHERE x.how = 'push') AS pushes
        FROM cv_w w JOIN cv_r r USING (k, src)
        JOIN raw.seed_sends x ON x.k = w.k AND x.src = w.src AND x.node = r.seed_node AND x.fate = 'written'
                              AND x.t_ms >= w.push_ms - 1000 AND x.t_ms < w.push_ms
        GROUP BY ALL),
    cv_m AS (
        SELECT w.k, w.src, w.seq, list_filter([
                   CASE WHEN w.children = 0 AND w.dishonest_children = 0 THEN 'uncovered' END,
                   CASE WHEN w.children = 0 AND w.dishonest_children > 0 THEN 'defectors_only' END,
                   CASE WHEN w.children > 0 AND w.push_ms IS NULL THEN 'not_written' END,
                   CASE WHEN w.children > 0 AND w.push_wait_ms > w.interval_ms THEN 'written_late' END,
                   CASE WHEN w.children > 0 AND w.push_wait_ms <= w.interval_ms
                             AND w.push_held_ms > w.pub_ms + w.interval_ms THEN 'delivered_late' END,
                   CASE WHEN w.children > 0 AND p.departed THEN 'departed' END], x -> x IS NOT NULL) AS m
        FROM cv_w w LEFT JOIN cv_departed p USING (k, src, seq))
    SELECT w.k, w.src, w.seq, w.stream, w.pub_ms, w.phase, w.children, w.dishonest_children,
           CASE WHEN len(m.m) > 0 THEN m.m ELSE ['other'] END AS matches,
           CASE WHEN len(m.m) > 0 THEN m.m[1] ELSE 'other' END AS cause,
           b.e.reason AS last_end, rm.d.kind AS seed_end, rm.d.why AS seed_why,
           CASE WHEN b.seq IS NOT NULL THEN dc.seq IS NOT NULL END AS declined, w.fates,
           CASE WHEN w.cancel_ms IS NOT NULL THEN coalesce(w.first_ms <= w.cancel_ms, false) END AS held_at_cancel,
           w.push_ms, w.push_wait_ms, w.push_held_ms - w.push_ms AS delivery_ms,
           CASE WHEN w.push_ms IS NOT NULL THEN coalesce(l.answer_bytes, 0) END AS answer_bytes,
           CASE WHEN w.push_ms IS NOT NULL THEN coalesce(l.push_bytes, 0) END AS push_bytes,
           CASE WHEN w.push_ms IS NOT NULL THEN coalesce(l.pushes, 0) END AS pushes
    FROM cv_w w JOIN cv_m m USING (k, src, seq) LEFT JOIN cv_bare b USING (k, src, seq)
    LEFT JOIN cv_removed rm USING (k, src, seq) LEFT JOIN cv_declined dc USING (k, src, seq)
    LEFT JOIN cv_load l USING (k, src, seq);

-- coverage_causes_in over the runs `jobs`.
-- needs: arrivals arrivals.released_ms arrivals.incarnation parents seed_decisions seed_sends events
CREATE OR REPLACE MACRO coverage_causes_of(jobs) AS TABLE
    FROM coverage_causes_in((SELECT list(cv_run) FROM coverage_runs_of(jobs) cv_run));

-- The bytes an access link (`spec`, run.sh's class: strims-sim's link.rs)
-- charges a packet past its IP bytes: tbf's and trace's an Ethernet header
-- (14), fiber's 18; none without a spec (a sim's uplink charging IP bytes
-- alone); NULL for cable (DOCSIS's 64-byte floor) and dsl (ATM's cells,
-- PTM's code), which charge no fixed overhead a packet.
-- `dflt` is the class a run without a spec has: run.sh's tbf for a testbed
-- run (twin_runs_of passes it), none for a sim's.
CREATE OR REPLACE MACRO link_overhead(spec, dflt := '') AS
    CASE split_part(coalesce(spec, dflt), ',', 1) WHEN '' THEN 0 WHEN 'tbf' THEN 14 WHEN 'trace' THEN 14
                                                  WHEN 'fiber' THEN 18 END;

-- The viewers' link_overhead of a run's knobs (`env`): their one class's
-- (STRIMS_SIM_CAL_VIEWER_LINK, else `dflt`), or STRIMS_SIM_CAL_LINKS'
-- where those give one overhead, else NULL.
CREATE OR REPLACE MACRO viewers_link_overhead(env, dflt := '') AS
    CASE WHEN env['STRIMS_SIM_CAL_LINKS'] IS NULL THEN link_overhead(env['STRIMS_SIM_CAL_VIEWER_LINK'], dflt := dflt)
         WHEN len(list_distinct(list_transform(string_split(trim(env['STRIMS_SIM_CAL_LINKS']), ' '),
                                               lambda x: link_overhead(split_part(x, ':', 2))))) = 1
         THEN link_overhead(split_part(string_split(trim(env['STRIMS_SIM_CAL_LINKS']), ' ')[1], ':', 2)) END;

-- twin_runs_of's count of the seed's meter samples of each run of the
-- runs `jobs` that record its runner's channel bytes (meter_samples'
-- channel_bytes, a testbed node's lines' field of that name), a row a run
-- with the seed's samples: k, src, samples, counted. A catalog whose meter
-- samples hold none (no run indexed records them) has no rows of it,
-- rather than a macro that fails: its runs count none.
-- needs: meter_samples.channel_bytes
-- else none: k VARCHAR, src VARCHAR, samples BIGINT, counted BIGINT
CREATE OR REPLACE MACRO channel_counted_of(jobs) AS TABLE
    SELECT m.k, m.src, count(*) AS samples, count(m.channel_bytes) AS counted FROM meter_samples m
    SEMI JOIN (SELECT DISTINCT b.k, b.src FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k)
        USING (k, src)
    WHERE m.role = 'seed'
    GROUP BY ALL;

-- twin_runs_of's window over each run of the runs `jobs`, in seconds of
-- stream: [lo_s, hi_s), its run's own stream's end where hi_s is below 0
-- (the default): the stream's length its job's knobs give
-- (STRIMS_SIM_CAL_SECS, calibration_star's 180 s where they lack it).
-- A run records seconds past its stream (a twin to its SECS + 10, a
-- testbed run to about SECS + 5), where its meter has nothing to measure
-- and its viewers' lag is carried 1000 ms a second, so a window to the
-- run's end scores its tail. k, src, lo, hi.
-- needs: jobs.env
CREATE OR REPLACE MACRO twin_window_of(jobs, lo_s := 10, hi_s := -1) AS TABLE
    SELECT b.k, b.src, lo_s::DOUBLE AS lo,
           CASE WHEN hi_s >= 0 THEN hi_s
                ELSE coalesce(TRY_CAST(any_value(b.env['STRIMS_SIM_CAL_SECS']) AS DOUBLE), 180) END::DOUBLE AS hi
    FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k
    GROUP BY b.k, b.src;

-- The testbed and its twin (simlab.testbed): each run of the jobs `jobs`
-- (a side: structs of a job's `k`, its `cell` and `seed`) over the stream's
-- seconds [lo_s, hi_s) (twin_window_of's: by default from 10 s, past the
-- startup's first seconds, to the end of each run's own stream, its
-- SECS), one row a measure, the same SQL over a testbed
-- run's tables and a sim run's: k, src, sim, cell, seed, measure, value,
-- basis (what a measure's definition took as given, NULL for nothing).
-- A starved run (no viewer read a frame: a testbed run, a collapsed twin)
-- is a result too, its lag measures carried across seconds without reads;
-- a measure a run's tables lack is left out. Bytes are in one of two
-- units, each side's from what it records: IP bytes (each packet's IP
-- length: a testbed capture's; a sim's link charge less its access link
-- class's overhead a packet, link_overhead) and link bytes (what the
-- access link charged: a sim's; a testbed capture's IP bytes and its
-- class's overhead a packet; a twin's acknowledgements, pure ACKs and
-- SACKs, are its link's charge too, as its data). Score a testbed run
-- against twins built from the tree that ingested it: these units are
-- the code's, and the lake can't tell a twin's binary's from its rows. A
-- run without a link spec has run.sh's tbf on the testbed and none in a
-- sim. A class whose charge isn't a fixed overhead a packet (cable's
-- floor, dsl's cells) leaves the side that would convert unmeasured. A
-- data channel's SACKs, which a testbed capture can't tell from its data
-- inside DTLS,
-- count in the uplink's bytes, packets and small packets on both sides.
-- The measures:
--   over_target_s       seconds in which an honest viewer lags past
--                       @{delay_target_ms} ms (viewer_lag_of's lag,
--                       carried across a second without reads)
--   lag_p99_ms          the p99 of the honest viewers' lag over their
--                       seconds (the same lag)
--   seed_upload         the seed's uplink IP bytes (bins' seed_up_bytes
--                       and its SACKs; TCP's pure ACKs apart) over its
--                       uplink's rate (its uplink_rate levels) times the
--                       window's bins that measure them (a testbed run's
--                       capture that stopped early leaves the rest
--                       unmeasured; each per-second rate below takes the
--                       bins that measure it alike)
--   seed_charged        its link bytes, its pure ACKs' too, over the same
--   seed_busy_permille  the seed meter's busy share (seconds' busy)
--   seed_copies         an adaptive seed's copies (seconds' copies)
--   seed_pushes_s, seed_answers_s
--                       the seed's chunks pushed, and sent otherwise, a
--                       second
--   viewer_hits_s, viewer_misses_s, viewer_pruned_s, viewer_switched_s,
--   viewer_shed_s       the honest viewers' pushes in time, deadlines
--                       released to pull, parents pruned, replaced and
--                       lost to a shed, a second (summed)
--   meter_p10, meter_p50, meter_p90
--                       the seed's meter samples' capacity over what it
--                       could upload then (meter_samples: avail_bps), as
--                       the row's meter_seed_p* takes them: the truth
--                       min(L x S_c, A x S_c x S_t) of the sim's
--                       report::upload_truth on both sides, a testbed
--                       run's from its capture and its nodes' channel
--                       bytes; one whose node lines don't record them
--                       (meter_samples' channel_bytes empty) takes S_c =
--                       1, and `basis` says so
--   truth_bps           that truth itself, in channel bytes a second: the
--                       mean over the seed's meter samples in the window
--                       that have one (avail_bps), an outage's 0 among
--                       them: as many as the seed took there, so a seed
--                       whose first viewer arrives past the window's start
--                       (a twin's trickle) averages fewer than its
--                       seconds; a testbed run's whose node lines don't
--                       record its channel bytes reads high by 1 / S_c,
--                       and `basis` says so
--   seed_drops_s        packets the seed's uplink queue dropped a second
--   seed_pps            packets the seed's uplink carried a second, pure
--                       ACKs in
--   seed_ack_share      of those, the pure ACKs' share
--   seed_small_share    of the others, the small ones' (<= 256 bytes)
--   viewer_up_bps       the viewers' uplink IP bytes a second (summed; a
--                       mix of viewer classes of two overheads leaves the
--                       sim's unmeasured)
--   competitor_share    over the seconds a competing upload ran (its
--                       competing_start to its competing_stop), the mean
--                       share of the seed's uplink rate it took (1 -
--                       seconds' avail over the rate): its IP bytes, which
--                       both sides count it in
--   competitor_charged  the same in link bytes: its class's overhead a
--                       packet more, at full packets (full_packet bytes)
--   seed_wire_per_written
--                       the seed's uplink IP bytes, pure ACKs and SACKs
--                       in, over the bytes its driver wrote (node_bins:
--                       data_bytes and ovh), over the bins its capture
--                       measured
-- and over the seed's meter samples (meter_samples, every one):
--   seed_rate_bps       its meter's rate, mean
--   seed_children, seed_budget, seed_copies_p50, seed_latest_floor_p50_ms
--                       its subscriptions served, an adaptive seed's budget,
--                       copies and the latest floor its budget check read,
--                       medians: each steps with joins and changes, and
--                       its median is the level most of the window holds,
--                       where a mean pools the ramp to it. The budget's
--                       median is over the samples with a measured
--                       budget: an unmeasured one is empty, or 0, which no
--                       measured budget is (it is a slot a stream at
--                       least). A seed takes meter samples from its first
--                       viewer's arrival on both sides (a twin's from the
--                       stats tick after it, a testbed node's from its
--                       line after it) and measures its budget about 3 s
--                       on, so a seed whose first viewer arrives past
--                       about 6 s has unmeasured samples in the window
--                       from 10 s; a twin's viewers arrive as its knobs
--                       draw them (a trickle by its seed), a testbed
--                       run's as its JOINS give them
--   seed_queue_delay_p50_ms, seed_queue_delay_p90_ms,
--   seed_busy_p50_permille, seed_busy_p90_permille
--                       its meter's queueing delay and busy share
--   seed_latest_under_10ms_share
--                       the share of samples whose links' latest round
--                       trips stand under 10 ms above their bases
--   seed_growth_share   the share of samples (with a rate) whose capacity
--                       reads growth_x (3.5) times its rate or more
--   seed_stream_requests_s
--                       STREAM_REQUESTs it answered a second (its meter's
--                       requests_in: subscriptions asked, not chunks)
-- and from bins and peer_samples:
--   seed_up_queue_p50_ms, seed_up_queue_p90_ms
--                       its uplink queue's wait at the bins' edges (a
--                       testbed run's tbf backlog over its nominal rate)
--   seed_rtt_p50_ms, seed_cto_p50_ms, viewer_seed_rtt_p50_ms,
--   viewer_rtt_p50_ms, viewer_cto_p50_ms
--                       the request windows' round trips and timeouts: the
--                       seed's channels', the viewers' to the seed, and the
--                       viewers' among themselves (samples with a round
--                       trip)
--   viewer_requests_s   the viewers' REQUESTs out a second (summed)
--   seed_requests_in_s  the REQUESTs the seed's peers sent it a second
--                       (summed over its channels' requests_in)
-- needs: seconds bins bins.seed_up_queue_ms node_bins meter_samples peer_samples viewer_seconds viewers events
-- needs: meter_samples.rate_bps meter_samples.busy_permille meter_samples.queue_delay_ms
-- needs: meter_samples.path_latest_ms meter_samples.children meter_samples.requests_in meter_samples.budget
-- needs: meter_samples.copies meter_samples.latest_floor_ms
CREATE OR REPLACE MACRO twin_runs_of(jobs, lo_s := 10, hi_s := -1, full_packet := 1500,
                                     growth_x := 3.5) AS TABLE
    WITH w AS (FROM twin_window_of(jobs, lo_s := lo_s, hi_s := hi_s)),
    j AS (
        SELECT DISTINCT u.cell, u.seed::BIGINT AS seed, b.k, b.src, b.sim, b.bin_ms,
               link_overhead(b.env['STRIMS_SIM_CAL_SEED_LINK'], dflt := CASE WHEN b.sim = 'testbed' THEN 'tbf' ELSE '' END)
                   AS seed_ovh,
               viewers_link_overhead(b.env, dflt := CASE WHEN b.sim = 'testbed' THEN 'tbf' ELSE '' END) AS viewer_ovh
        FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k),
    lag AS (
        SELECT l.k, l.src, l.sec, max(l.lag_ms) AS lag_ms
        FROM viewer_lag_of(jobs) l JOIN w USING (k, src) WHERE l.sec >= w.lo AND l.sec < w.hi GROUP BY ALL),
    lags AS (
        SELECT k, src, count(*) FILTER (WHERE lag_ms > @{delay_target_ms}) AS over_target_s FROM lag GROUP BY ALL),
    p99 AS (
        SELECT l.k, l.src, sketch_quantile(l.lag_ms, 99) AS lag_p99_ms
        FROM viewer_lag_of(jobs) l JOIN w USING (k, src) WHERE l.sec >= w.lo AND l.sec < w.hi GROUP BY ALL),
    sec AS (
        SELECT s.k, s.src, count(*) AS n, avg(s.busy) AS seed_busy_permille, avg(s.copies) AS seed_copies,
               avg(s.pushed) AS seed_pushes_s, avg(s.sent - s.pushed) AS seed_answers_s,
               avg(s.hits) AS viewer_hits_s, avg(s.misses) AS viewer_misses_s, avg(s.pruned) AS viewer_pruned_s
        FROM seconds s JOIN w USING (k, src) WHERE s.sec >= w.lo AND s.sec < w.hi GROUP BY ALL),
    rate AS (
        SELECT e.k, e.src, e.t_us, e.value AS rate
        FROM events e SEMI JOIN j USING (k, src) WHERE e.kind = 'uplink_rate' AND e.node = 0),
    bin AS (
        SELECT b.k, b.src, r.rate, b.seed_up_bytes + coalesce(b.seed_up_sack_bytes, 0) AS seed_up_bytes,
               b.seed_up_packets + coalesce(b.seed_up_sack_packets, 0) AS seed_up_packets,
               b.seed_small_packets + coalesce(b.seed_up_sack_packets, 0) AS seed_small_packets,
               b.seed_up_ack_packets, b.seed_up_ack_bytes, b.seed_drops,
               b.viewer_up_bytes + coalesce(b.viewer_up_sack_bytes, 0) AS viewer_up_bytes,
               b.viewer_up_packets + coalesce(b.viewer_up_sack_packets, 0) AS viewer_up_packets, b.viewer_switched,
               b.viewer_shed, j.bin_ms, j.sim, j.seed_ovh, j.viewer_ovh
        FROM bins b JOIN j USING (k, src) JOIN w USING (k, src)
        ASOF LEFT JOIN rate r ON r.k = b.k AND r.src = b.src AND b.t_ms * 1000 >= r.t_us
        WHERE b.t_ms >= w.lo * 1000 AND b.t_ms < w.hi * 1000),
    -- Each side's bytes in IP bytes and link bytes (see the docs): a
    -- testbed's recorded IP, a sim's link (its acknowledgements' too).
    units AS (
        SELECT *,
               CASE WHEN sim = 'testbed' THEN seed_up_bytes ELSE seed_up_bytes - seed_ovh * seed_up_packets END
                   AS seed_ip,
               CASE WHEN sim = 'testbed'
                    THEN seed_up_bytes + seed_up_ack_bytes + seed_ovh * (seed_up_packets + seed_up_ack_packets)
                    ELSE seed_up_bytes + seed_up_ack_bytes END AS seed_link,
               CASE WHEN sim = 'testbed' THEN viewer_up_bytes ELSE viewer_up_bytes - viewer_ovh * viewer_up_packets END
                   AS viewer_ip
        FROM bin),
    -- Each rate is over the bins that measure what it counts: a testbed
    -- run's capture or tc sampler that stopped early leaves its bins past
    -- the stop unmeasured (NULL), which count in neither its bytes nor its
    -- time. The viewers' switched and shed are their nodes' own counters,
    -- which every bin measures.
    wire AS (
        SELECT k, src, count(*) * any_value(bin_ms) / 1000.0 AS secs,
               sum(seed_ip) / sum(rate * bin_ms / 1000.0) FILTER (WHERE seed_ip IS NOT NULL) AS seed_upload,
               sum(seed_link) / sum(rate * bin_ms / 1000.0) FILTER (WHERE seed_link IS NOT NULL) AS seed_charged,
               sum(seed_drops) / (count(seed_drops) * any_value(bin_ms) / 1000.0) AS seed_drops_s,
               sum(seed_up_packets + seed_up_ack_packets)
                   / (count(seed_up_packets + seed_up_ack_packets) * any_value(bin_ms) / 1000.0) AS seed_pps,
               sum(seed_up_ack_packets) / nullif(sum(seed_up_packets) + sum(seed_up_ack_packets), 0) AS seed_ack_share,
               sum(seed_small_packets) / nullif(sum(seed_up_packets), 0) AS seed_small_share,
               sum(viewer_ip) / (count(viewer_ip) * any_value(bin_ms) / 1000.0) AS viewer_up_bps,
               sum(viewer_switched) / (count(viewer_switched) * any_value(bin_ms) / 1000.0) AS viewer_switched_s,
               sum(viewer_shed) / (count(viewer_shed) * any_value(bin_ms) / 1000.0) AS viewer_shed_s
        FROM units GROUP BY ALL),
    -- The meter's truth, what the seed could upload at each sample: its
    -- meter_samples' avail_bps (Report::upload_cap_at's, on both sides),
    -- truth(t) = min(L x S_c, A x S_c x S_t) in channel bytes a second:
    -- A what its link carries for it over (t - 1 s, t] less its
    -- competitor's link charge, L its QoS limit, S_t its transports' share
    -- of its link over its near-saturated windows, S_c its runner's
    -- channel bytes over vnic's. A testbed run's A and S_t come from its
    -- tc rates, steps and capture (a data channel's S_t from its
    -- datagrams' lengths under its stack's fixed headers over their charge,
    -- SACKs alone and dropped datagrams apart), and its S_c from its nodes'
    -- lines
    -- (channel_bytes, bytes_written: strims-sim's testbed::gate). A run whose
    -- node lines don't record its channel bytes (channel_bytes empty)
    -- takes S_c as 1, so its truth reads high, by 1 / S_c, and its
    -- readings over it (capacity over truth) read low: `basis` says so.
    truth AS (
        SELECT m.k, m.src, m.t_ms, m.capacity_bps, m.avail_bps AS truth_bps
        FROM meter_samples m JOIN w USING (k, src)
        WHERE m.role = 'seed' AND m.t_ms >= w.lo * 1000 AND m.t_ms < w.hi * 1000),
    meter AS (
        SELECT k, src, sketch_quantiles(capacity_bps / truth_bps, [10, 50, 90]) AS q
        FROM truth WHERE capacity_bps > 0 AND truth_bps > 0 GROUP BY ALL),
    tr AS (SELECT k, src, avg(truth_bps) AS truth_bps FROM truth GROUP BY ALL),
    -- The runs whose seed's every sample records its channel bytes.
    counted AS (SELECT k, src, true AS all_counted FROM channel_counted_of(jobs) WHERE counted = samples),
    competing AS (
        SELECT e.k, e.src, e.t_us AS start_us,
               lead(e.t_us) OVER (PARTITION BY e.k, e.src ORDER BY e.t_us, e.kind) AS stop_us, e.kind
        FROM events e SEMI JOIN j USING (k, src)
        WHERE e.node = 0 AND e.kind IN ('competing_start', 'competing_stop')),
    comp AS (
        SELECT s.k, s.src, avg(1 - s.avail / r.rate) AS competitor_share,
               avg(1 - s.avail / r.rate) * (full_packet + any_value(j.seed_ovh)) / full_packet AS competitor_charged
        FROM seconds s JOIN j USING (k, src) JOIN w USING (k, src)
        JOIN competing c ON c.k = s.k AND c.src = s.src AND c.kind = 'competing_start'
         AND (s.sec + 0.5) * 1e6 >= c.start_us AND (s.sec + 0.5) * 1e6 < c.stop_us
        ASOF JOIN rate r ON r.k = s.k AND r.src = s.src AND (s.sec + 0.5) * 1e6 >= r.t_us
        WHERE s.sec >= w.lo AND s.sec < w.hi AND s.avail >= 0
        GROUP BY ALL),
    -- The seed's wire bytes over what its driver wrote, over the bins its
    -- capture measured (its driver's counters measure every bin).
    nb AS (
        SELECT k, src, sum(wire) / nullif(sum(written) FILTER (WHERE wire IS NOT NULL), 0) AS seed_wire_per_written
        FROM (SELECT n.k, n.src,
                     n.up_bytes + coalesce(n.up_sack_bytes, 0) + n.up_ack_bytes
                         - CASE WHEN j.sim = 'testbed' THEN 0
                                ELSE j.seed_ovh * (n.up_packets + coalesce(n.up_sack_packets, 0) + n.up_ack_packets)
                           END AS wire,
                     n.data_bytes + n.ovh AS written
              FROM node_bins n JOIN j USING (k, src) JOIN w USING (k, src)
              WHERE n.node = 0 AND n.t_ms >= w.lo * 1000 AND n.t_ms < w.hi * 1000)
        GROUP BY ALL),
    ms AS (
        SELECT m.k, m.src, avg(m.rate_bps) AS seed_rate_bps, sketch_quantile(m.children, 50) AS seed_children,
               sketch_quantile(nullif(m.budget, 0), 50) AS seed_budget, sketch_quantile(m.copies, 50) AS seed_copies_p50,
               sketch_quantile(m.queue_delay_ms, 50) AS seed_queue_delay_p50_ms,
               sketch_quantile(m.queue_delay_ms, 90) AS seed_queue_delay_p90_ms,
               sketch_quantile(m.busy_permille, 50) AS seed_busy_p50_permille,
               sketch_quantile(m.busy_permille, 90) AS seed_busy_p90_permille,
               sketch_quantile(m.latest_floor_ms, 50) AS seed_latest_floor_p50_ms,
               avg((m.path_latest_ms < 10)::INT) AS seed_latest_under_10ms_share,
               avg(CASE WHEN m.rate_bps > 0 THEN (m.capacity_bps >= growth_x * m.rate_bps)::INT END)
                   AS seed_growth_share,
               (max(m.requests_in) - min(m.requests_in)) / nullif((max(m.t_ms) - min(m.t_ms)) / 1000.0, 0)
                   AS seed_stream_requests_s
        FROM meter_samples m JOIN w USING (k, src)
        WHERE m.role = 'seed' AND m.t_ms >= w.lo * 1000 AND m.t_ms < w.hi * 1000 GROUP BY ALL),
    q AS (
        SELECT b.k, b.src, sketch_quantile(b.seed_up_queue_ms, 50) AS seed_up_queue_p50_ms,
               sketch_quantile(b.seed_up_queue_ms, 90) AS seed_up_queue_p90_ms
        FROM bins b JOIN w USING (k, src) WHERE b.t_ms >= w.lo * 1000 AND b.t_ms < w.hi * 1000 GROUP BY ALL),
    ps AS (
        SELECT p.k, p.src,
               sketch_quantile(CASE WHEN p.node = 0 THEN p.rtt_ms END, 50) AS seed_rtt_p50_ms,
               sketch_quantile(CASE WHEN p.node = 0 THEN p.cto_ms END, 50) AS seed_cto_p50_ms,
               sketch_quantile(CASE WHEN p.node <> 0 AND p.peer = 0 THEN p.rtt_ms END, 50) AS viewer_seed_rtt_p50_ms,
               sketch_quantile(CASE WHEN p.node <> 0 AND p.peer <> 0 THEN p.rtt_ms END, 50) AS viewer_rtt_p50_ms,
               sketch_quantile(CASE WHEN p.node <> 0 AND p.peer <> 0 THEN p.cto_ms END, 50) AS viewer_cto_p50_ms
        FROM peer_samples p JOIN w USING (k, src)
        WHERE p.t_ms >= w.lo * 1000 AND p.t_ms < w.hi * 1000 AND p.rtt_ms > 0 GROUP BY ALL),
    pr0 AS (
        SELECT p.k, p.src, p.node, p.peer, max(p.requests_out) - min(p.requests_out) AS n,
               max(p.requests_in) - min(p.requests_in) AS n_in
        FROM peer_samples p JOIN w USING (k, src)
        WHERE p.t_ms >= w.lo * 1000 AND p.t_ms < w.hi * 1000 GROUP BY ALL),
    pspan AS (
        SELECT p.k, p.src, (max(p.t_ms) - min(p.t_ms)) / 1000.0 AS span
        FROM peer_samples p JOIN w USING (k, src)
        WHERE p.t_ms >= w.lo * 1000 AND p.t_ms < w.hi * 1000 GROUP BY ALL),
    pr AS (
        SELECT k, src, sum(n) FILTER (WHERE node <> 0) / nullif(any_value(span), 0) AS viewer_requests_s,
               sum(n_in) FILTER (WHERE node = 0) / nullif(any_value(span), 0) AS seed_requests_in_s
        FROM pr0 JOIN pspan USING (k, src) GROUP BY ALL),
    wide AS (
        SELECT j.k, j.src, j.sim, j.cell, j.seed, lags.over_target_s::DOUBLE AS over_target_s,
               p99.lag_p99_ms::DOUBLE AS lag_p99_ms, sec.* EXCLUDE (k, src, n), wire.* EXCLUDE (k, src, secs),
               meter.q[1] AS meter_p10, meter.q[2] AS meter_p50, meter.q[3] AS meter_p90, tr.truth_bps,
               comp.competitor_share, comp.competitor_charged, nb.seed_wire_per_written,
               ms.* EXCLUDE (k, src), q.* EXCLUDE (k, src), ps.* EXCLUDE (k, src), pr.viewer_requests_s,
               pr.seed_requests_in_s, counted.all_counted IS NULL AS sc_one
        FROM j LEFT JOIN lags USING (k, src) LEFT JOIN p99 USING (k, src) LEFT JOIN sec USING (k, src)
        LEFT JOIN wire USING (k, src) LEFT JOIN meter USING (k, src) LEFT JOIN tr USING (k, src)
        LEFT JOIN comp USING (k, src)
        LEFT JOIN nb USING (k, src) LEFT JOIN ms USING (k, src) LEFT JOIN q USING (k, src)
        LEFT JOIN ps USING (k, src) LEFT JOIN pr USING (k, src) LEFT JOIN counted USING (k, src))
    SELECT k, src, sim, cell, seed, measure, value::DOUBLE AS value,
           CASE WHEN sim = 'testbed' AND (measure LIKE 'meter_p%' OR measure = 'truth_bps') AND sc_one
                THEN 'truth with S_c = 1 (no channel_bytes)' END AS basis
    FROM (UNPIVOT wide ON COLUMNS(* EXCLUDE (k, src, sim, cell, seed, sc_one)) INTO NAME measure VALUE value)
    WHERE value IS NOT NULL;

-- The testbed runs whose source matches the LIKE pattern src_like (a
-- farm run's farm/<run>/%, a local ingest's testbed/<name>), as a side:
-- {cell, seed, k, src}, a run's seed its repeat (a twin's seed pairs with
-- the run of its number). `kind` keeps 'native' or 'browsers' runs
-- (STRIMS_SIM_RTC set), and `arm` one arm's (jobs.stored_arm); 'one', each
-- one's default, takes the runs as they are but fails where they hold two
-- kinds or two arms, which a side mustn't pool; 'any' pools them: two
-- arms' runs at one cell and seed are both the side's, two runs of the
-- cell (each its own k: twin_fit_rows' medians and places take both).
-- Another kind, an arm with no run, and two runs at one cell, arm and seed
-- (which twin_runs_of would pool) fail.
-- needs: jobs.sim jobs.env jobs.stored_arm
CREATE OR REPLACE MACRO testbed_side(src_like, kind := 'one', arm := 'one') AS (
    WITH r AS (
        SELECT stored_cell, seed, k, src, stored_arm, env['STRIMS_SIM_RTC'] IS NOT NULL AS browsers
        FROM jobs
        WHERE sim = 'testbed' AND src LIKE src_like
          AND (kind IN ('one', 'any') OR browsers = (kind = 'browsers'))
          AND (arm IN ('one', 'any') OR stored_arm = arm))
    SELECT CASE
        WHEN kind NOT IN ('one', 'any', 'native', 'browsers')
            THEN error('testbed_side: kind ' || kind || ' is none of one, any, native and browsers')
        WHEN arm NOT IN ('one', 'any') AND runs = 0
            THEN error('testbed_side: no testbed run of arm ' || arm || ' in ' || src_like)
        WHEN kind = 'one' AND kinds > 1
            THEN error('testbed_side: native and browser runs, name a kind (native, browsers or any)')
        WHEN arm = 'one' AND arms > 1
            THEN error('testbed_side: runs of ' || arms || ' arms, name an arm or any')
        WHEN keys < runs
            THEN error('testbed_side: ' || (runs - keys) || ' runs share a cell, arm and seed with another '
                       || '(two matrices of one knobs, or settings the keying leaves out): narrow src_like')
        ELSE side END
    FROM (SELECT count(DISTINCT browsers) AS kinds, count(DISTINCT stored_arm) AS arms, count(*) AS runs,
                 count(DISTINCT (stored_cell, stored_arm, seed)) AS keys,
                 coalesce(list({'cell': stored_cell, 'seed': seed, 'k': k, 'src': src} ORDER BY stored_cell, seed),
                          []) AS side
          FROM r));

-- Twin rows against testbed rows (`a_rows` the twin's, `b_rows` the
-- testbed's: lists of {cell, measure, k, value, basis}, a run's measure
-- each, as twin_runs_of gives them): each cell and measure both sides have,
-- a_runs and b_runs, a_median and b_median (each side's median over its
-- runs, quantile_cont's: the mean of the middle two at an even count, as
-- a testbed's two repeats are; a sketch's p50 would take the lower),
-- ratio (b_median / a_median) and b_quantile, where the testbed's runs
-- stand among the twin's seeds: each run's mid-rank quantile among them
-- (the share of seeds below it, half of those it ties), their mean (0.5 a
-- twin centred on the testbed, 0 or 1 one wholly above or below it); and
-- over the cells, a row of cell NULL per measure: cells, corr (Pearson's,
-- of the cells' medians), rank_corr (Spearman's: of their ranks, ties
-- their mean rank), ratio (the geometric mean of the cells' ratios) and
-- b_quantile (the cells' mean). How well the twin follows the testbed
-- across cells is the fidelity a model change is judged by. `caveat`
-- flags a fit whose testbed runs took something as given (their rows'
-- `basis`, twin_runs_of's: a truth that takes its channel share as 1, the
-- truth's own, truth_bps, and a meter reading's over it, meter_p*), saying
-- how many of them did.
CREATE OR REPLACE MACRO twin_fit_rows(a_rows, b_rows) AS TABLE
    WITH a AS (SELECT unnest(a_rows, recursive := true)),
    b AS (SELECT unnest(b_rows, recursive := true)),
    am AS (SELECT cell, measure, count(*) AS runs, quantile_cont(value, 0.5) AS m FROM a GROUP BY ALL),
    bm AS (SELECT cell, measure, count(*) AS runs, quantile_cont(value, 0.5) AS m, count(basis) AS given,
                  string_agg(DISTINCT basis, '; ' ORDER BY basis) AS basis FROM b GROUP BY ALL),
    place AS (
        SELECT b.cell, b.measure, b.k,
               (count(*) FILTER (WHERE a.value < b.value) + 0.5 * count(*) FILTER (WHERE a.value = b.value))
                   / count(*) AS q
        FROM b JOIN a USING (cell, measure) GROUP BY b.cell, b.measure, b.k),
    pq AS (SELECT cell, measure, avg(q) AS q FROM place GROUP BY ALL),
    c AS (
        SELECT cell, measure, am.runs AS a_runs, bm.runs AS b_runs, am.m AS a_median, bm.m AS b_median,
               CASE WHEN am.m <> 0 THEN bm.m / am.m END AS ratio, pq.q AS b_quantile, bm.given, bm.basis
        FROM am JOIN bm USING (cell, measure) JOIN pq USING (cell, measure)),
    ranked AS (
        SELECT *,
               rank() OVER (PARTITION BY measure ORDER BY a_median)
                   + (count(*) OVER (PARTITION BY measure, a_median) - 1) / 2.0 AS ra,
               rank() OVER (PARTITION BY measure ORDER BY b_median)
                   + (count(*) OVER (PARTITION BY measure, b_median) - 1) / 2.0 AS rb FROM c)
    SELECT * EXCLUDE (given, basis),
           CASE WHEN given > 0 THEN 'the testbed''s ' || basis || ' in ' || given || ' of its ' || b_runs || ' runs' END
               AS caveat
    FROM (
        SELECT cell, measure, a_runs, b_runs, a_median, b_median, ratio, b_quantile, NULL::BIGINT AS cells,
               NULL::DOUBLE AS corr, NULL::DOUBLE AS rank_corr, given, basis
        FROM c
        UNION ALL
        SELECT NULL, measure, sum(a_runs), sum(b_runs), NULL, NULL,
               exp(avg(ln(CASE WHEN ratio > 0 THEN ratio END))), avg(b_quantile), count(*), corr(a_median, b_median),
               corr(ra, rb), sum(given), string_agg(DISTINCT basis, '; ' ORDER BY basis)
        FROM ranked GROUP BY measure);

-- twin_fit_rows of a testbed side and its twin's (twin_runs_of over both,
-- `a_jobs` the twin's, `b_jobs` the testbed's), each run over its own
-- window (twin_window_of: by default its stream from 10 s).
-- needs: seconds bins bins.seed_up_queue_ms node_bins meter_samples peer_samples viewer_seconds viewers events
-- needs: meter_samples.rate_bps meter_samples.busy_permille meter_samples.queue_delay_ms
-- needs: meter_samples.path_latest_ms meter_samples.children meter_samples.requests_in meter_samples.budget
-- needs: meter_samples.copies meter_samples.latest_floor_ms
CREATE OR REPLACE MACRO twin_fit_of(a_jobs, b_jobs, lo_s := 10, hi_s := -1, full_packet := 1500,
                                    growth_x := 3.5) AS TABLE
    FROM twin_fit_rows(
        (SELECT coalesce(list({'cell': cell, 'measure': measure, 'k': k, 'value': value, 'basis': basis}),
                         []::STRUCT(cell VARCHAR, measure VARCHAR, k VARCHAR, value DOUBLE, basis VARCHAR)[])
         FROM twin_runs_of(a_jobs, lo_s := lo_s, hi_s := hi_s,
                           full_packet := full_packet, growth_x := growth_x)),
        (SELECT coalesce(list({'cell': cell, 'measure': measure, 'k': k, 'value': value, 'basis': basis}),
                         []::STRUCT(cell VARCHAR, measure VARCHAR, k VARCHAR, value DOUBLE, basis VARCHAR)[])
         FROM twin_runs_of(b_jobs, lo_s := lo_s, hi_s := hi_s,
                           full_packet := full_packet, growth_x := growth_x)));
