-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- suite.py's scoring as the catalog's macros: what its scorecards, arm
-- scores and races compute, here once (suite.py prints what they return).
-- catalog.py loads them (scoring_sql) after what it makes itself:
-- metric_defs, job_runs, job_metrics and job_metrics_of from suite.py's
-- METRICS, and t975, Student's t. It fills in each @{...}: a constant of
-- suite.py's, analyze.py's or its own, or a row field (r.<field>), the
-- field where the rows have it, else NULL. A macro's `-- needs:` line names
-- the tables (or table.column) it reads that a catalog indexed by older
-- code may lack: a session's own objects over such a catalog make it one
-- that fails saying so (catalog.needed). A table macro read as a fallback
-- gives instead, with an `-- else none:` line of its columns, those
-- columns and no rows.
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

-- The hypergeometric probability of x of hits_ in the first n_ of n_ + n2_.
CREATE OR REPLACE MACRO hypergeom_p(x_, n_, n2_, hits_) AS
    exp(lgamma(n_ + 1) - lgamma(x_ + 1) - lgamma(n_ - x_ + 1)
        + lgamma(n2_ + 1) - lgamma(hits_ - x_ + 1) - lgamma(n2_ - hits_ + x_ + 1)
        - lgamma(n_ + n2_ + 1) + lgamma(hits_ + 1) + lgamma(n_ + n2_ - hits_ + 1));

-- The two-sided Fisher exact p of k_ in n_ against m_ in n2_: the
-- probabilities, in order, of every split no likelier than the one seen.
CREATE OR REPLACE MACRO fisher_p(k_, n_, m_, n2_) AS
    least(1.0::DOUBLE, list_reduce(list_filter(
        list_transform(range(greatest(0, k_ + m_ - n2_), least(n_, k_ + m_) + 1),
                       lambda x: hypergeom_p(x, n_, n2_, k_ + m_)),
        lambda pr: pr <= hypergeom_p(k_, n_, n2_, k_ + m_) * (1 + 1e-9)), lambda acc, pr: acc + pr));

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
-- pairs (the seeds paired), a_median and b_median (each side's upper
-- median over them), n (the pairs it is over: a ratio's those with both
-- values above 0), effect (the geometric mean ratio, exp of the mean
-- ln(b/a), or the mean difference b - a), lo and hi (its 95 % t-interval;
-- one pair gives 0 or -inf to inf), mark (effect_mark; h for a host
-- metric that moved) and metric_defs' columns.
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
               list_sort(list(a_value))[count(*) // 2 + 1] AS a_median,
               list_sort(list(b_value))[count(*) // 2 + 1] AS b_median,
               list(x ORDER BY seed) FILTER (WHERE x IS NOT NULL) AS xs
        FROM p GROUP BY cell, metric),
    s AS (SELECT *, len(xs) AS n, list_reduce(xs, lambda acc, y: acc + y) / len(xs) AS mean FROM g),
    h AS (
        SELECT *, CASE WHEN n >= 2 THEN t975_at(n - 1)
                       * sqrt(list_reduce(list_transform(xs, lambda y: (y - mean) ** 2), lambda acc, y: acc + y)
                              / (n - 1)) / sqrt(n) END AS half
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
--                lag p99 (a) under lag_ceiling and the arm's (b) not;
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
    lags AS (
        SELECT cell, list_sort(list(a_value)) AS av, list_sort(list(b_value)) AS bv
        FROM pairs_of(a_jobs, b_jobs) WHERE metric = 'lag_p99_ms' AND list_contains(cell_list, cell) GROUP BY cell),
    mid AS (
        SELECT cell,
               CASE WHEN len(av) % 2 = 1 THEN av[len(av) // 2 + 1] ELSE (av[len(av) // 2] + av[len(av) // 2 + 1]) / 2 END AS a_mid,
               CASE WHEN len(bv) % 2 = 1 THEN bv[len(bv) // 2 + 1] ELSE (bv[len(bv) // 2] + bv[len(bv) // 2 + 1]) / 2 END AS b_mid
        FROM lags),
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
               exp(list_reduce(list(ln(lag) ORDER BY cell_ord) FILTER (WHERE lag IS NOT NULL), lambda acc, y: acc + y)
                   / count(lag)) AS p99,
               exp(list_reduce(list(ln(wire) ORDER BY cell_ord) FILTER (WHERE wire IS NOT NULL), lambda acc, y: acc + y)
                   / count(wire)) AS wire,
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
-- +1000 ms a second without one, as analyze.py's lags() does: NULL before
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
--   stalls_per_min_<D>,      the rows' playback at delay D (the mean over
--   stalled_share_<D>,       runs; NULL in rows from before the sims
--   startup_ms_<D>           played them);
--   unhealthy_max_s,         the longest unhealthy spell (episodes) of any
--   unhealthy_p50_s          run, and the median of each run's longest;
-- for D in @{delay_target_ms} and @{delay_max_ms} (analyze.py's
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
               quantile_cont(p.lag_ms, 0.99) FILTER (WHERE p.phase = 'startup') AS lag_p99_startup_ms,
               quantile_cont(p.lag_ms, 0.99) FILTER (WHERE p.phase = 'steady') AS lag_p99_steady_ms,
               quantile_cont(p.lag_ms, 0.99) FILTER (WHERE starts_with(p.phase, 'after_')) AS lag_p99_after_ms
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
               avg(@{r.stalls_per_min_@{delay_target_ms}}) AS stalls_per_min_@{delay_target_ms},
               avg(@{r.stalls_per_min_@{delay_max_ms}}) AS stalls_per_min_@{delay_max_ms},
               avg(@{r.stalled_share_@{delay_target_ms}}) AS stalled_share_@{delay_target_ms},
               avg(@{r.stalled_share_@{delay_max_ms}}) AS stalled_share_@{delay_max_ms},
               avg(@{r.startup_ms_@{delay_target_ms}}) AS startup_ms_@{delay_target_ms},
               avg(@{r.startup_ms_@{delay_max_ms}}) AS startup_ms_@{delay_max_ms},
               coalesce(max(u.longest), 0) AS unhealthy_max_s, median(coalesce(u.longest, 0)) AS unhealthy_p50_s
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
               quantile_cont(p.lag_ms, 0.5) AS lag_p50_ms, quantile_cont(p.lag_ms, 0.9) AS lag_p90_ms,
               quantile_cont(p.lag_ms, 0.99) AS lag_p99_ms, max(p.lag_ms) AS lag_max_ms
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
           count(h.lag_ms) AS viewer_s, quantile_cont(h.lag_ms, 0.5) AS lag_p50_ms,
           quantile_cont(h.lag_ms, 0.9) AS lag_p90_ms, quantile_cont(h.lag_ms, 0.99) AS lag_p99_ms
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
-- series_meta's `streams` or the ppspp sim's row's `streams`:
-- `series_meta`, `row`; else inferred, one more than the greatest stream
-- its parents name, `parents`, else its asks', `asks`, else one, NULL;
-- a chunk's stream is its seq mod streams), interval_ms (the run's chunk
-- interval: the median gap between consecutive publications within a
-- stream, the seed's own rows), end_ms (the end of its last second in
-- `seconds`, or its last hold if that is later: strims-sim runs on past
-- its stream), and arrivals_rows and parents_rows (the run's rows of
-- each the catalog holds: without arrivals a run has no chunks, without
-- parents its I1 and its late chunks' classes are NULL).
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
        SELECT k, src, median(gap) AS interval_ms FROM (
            SELECT k, src, pub_ms - lag(pub_ms) OVER (PARTITION BY k, src, stream ORDER BY seq) AS gap FROM cv_pub)
        WHERE gap IS NOT NULL GROUP BY ALL),
    cv_secs AS (
        SELECT k, src, (max(sec) + 1) * 1000 AS end_ms FROM seconds SEMI JOIN cv_job_keys USING (k, src)
        GROUP BY ALL),
    cv_holds AS (
        SELECT k, src, max(t_ms) AS end_ms, count(*) AS rows_n FROM arrivals SEMI JOIN cv_job_keys USING (k, src)
        GROUP BY ALL)
    SELECT j.k, j.src, s.node AS seed_node, s.seed_from, t.streams, t.streams_from, g.interval_ms,
           greatest(e.end_ms, h.end_ms) AS end_ms, coalesce(h.rows_n, 0) AS arrivals_rows,
           coalesce(p.rows_n, 0) AS parents_rows
    FROM cv_job_keys j LEFT JOIN cv_seed s USING (k, src) LEFT JOIN cv_streams t USING (k, src)
    LEFT JOIN cv_gaps g USING (k, src) LEFT JOIN cv_secs e USING (k, src) LEFT JOIN cv_holds h USING (k, src)
    LEFT JOIN cv_parent_streams p USING (k, src);

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

-- The seed's child subscriptions in the runs `runs` (coverage_runs_of's
-- rows), from parents: k, src, node, stream, open_ms (a row naming the
-- seed the node's parent of the stream), close_ms (the node's next row of
-- the stream, else its run's end) and end_reason (that row's reason,
-- NULL to the run's end). A node's rows of a stream at one ms are ordered
-- as they can have happened: a @{reason_switch} first, an @{reason_open}
-- after the ends if the node had a parent before that ms (an open needs
-- none) and before them if not. A subscription of no length is none.
-- needs: parents
CREATE OR REPLACE MACRO seed_children_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
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
    SELECT k, src, node, stream, open_ms, close_ms, end_reason FROM cv_spans
    WHERE parent = seed_node AND close_ms > open_ms;

-- seed_children_in over the runs `jobs`.
-- needs: arrivals parents
CREATE OR REPLACE MACRO seed_children_of(jobs) AS TABLE
    FROM seed_children_in((SELECT list(cv_run) FROM coverage_runs_of(jobs) cv_run));

-- Each chunk the seed of a run of `runs` (coverage_runs_of's rows)
-- published, a row a chunk: k, src, seq, stream, pub_ms (the seed's own
-- arrivals row), phase (the run's, verdict_seconds', of its publication
-- second; @{tail} past the verdict's seconds), first_ms, first_node
-- and latency_ms (first_ms - pub_ms) of its earliest hold by any node but
-- the seed (ties by node, any incarnation), route (that hold's `push` or
-- `answer`, else `other`, or `never`: no node but the seed held it),
-- source (`seed`, `peer` or `unnamed`, the hold's peer: a first copy from
-- a node but the seed, or naming none, is an anomaly; NULL for never),
-- interval_ms (its run's), judged (published at least @{delay_max_ms} ms,
-- the latency budget's most, before its run's end: every other chunk,
-- held or not, is censored), late (never held, or held more than its
-- run's interval after its publication) and, for a judged late chunk of a
-- run with parents rows (NULL for the others), the seed's children of
-- its stream at its publication (seed_children_in's): children (how
-- many), class (the first that holds: no_seed_child; released, a child's
-- hold names a release to pull, arrivals' released_ms; pushed_late, a
-- child holds it by the seed's push; child_never, no child ever held it;
-- other) and after (for no_seed_child, how the stream's last seed child
-- before it ended, end_reason's; NULL for a stream that never had one).
-- needs: arrivals arrivals.released_ms parents
CREATE OR REPLACE MACRO coverage_chunks_in(runs) AS TABLE
    WITH cv_r AS (SELECT unnest(runs, recursive := true)),
    cv_arr AS MATERIALIZED (
        SELECT x.k, x.src, x.node, x.seq, x.t_ms, x.how, x.peer, x.released_ms, x.node = r.seed_node AS seed
        FROM arrivals x JOIN cv_r r USING (k, src)),
    cv_pub AS (
        SELECT x.k, x.src, x.seq, x.seq % r.streams AS stream, min(x.t_ms) AS pub_ms
        FROM cv_arr x JOIN cv_r r USING (k, src) WHERE x.seed AND x.how = 'own' GROUP BY ALL),
    cv_first AS (
        SELECT k, src, seq, arg_min({'t_ms': t_ms, 'how': how, 'peer': peer, 'node': node}, (t_ms, node)) AS h
        FROM cv_arr WHERE NOT seed GROUP BY ALL),
    cv_chunks_of AS (
        SELECT p.k, p.src, p.seq, p.stream, p.pub_ms, coalesce(v.phase, @{tail}) AS phase,
               f.h.t_ms AS first_ms, f.h.node AS first_node, f.h.t_ms - p.pub_ms AS latency_ms,
               CASE WHEN f.h IS NULL THEN 'never' WHEN f.h.how IN ('push', 'answer') THEN f.h.how
                    ELSE 'other' END AS route,
               CASE WHEN f.h IS NULL THEN NULL WHEN f.h.peer = r.seed_node THEN 'seed'
                    WHEN f.h.peer >= 0 THEN 'peer' ELSE 'unnamed' END AS source,
               r.interval_ms, p.pub_ms + @{delay_max_ms} <= r.end_ms AS judged,
               f.h IS NULL OR f.h.t_ms - p.pub_ms > r.interval_ms AS late, r.parents_rows > 0 AS parented
        FROM cv_pub p JOIN cv_r r USING (k, src) LEFT JOIN cv_first f USING (k, src, seq)
        LEFT JOIN verdict_seconds v ON v.k = p.k AND v.src = p.src AND v.sec = p.pub_ms // 1000),
    cv_subs AS (FROM seed_children_in(runs)),
    cv_kids AS (
        SELECT c.k, c.src, c.seq, s.node FROM cv_chunks_of c
        JOIN cv_subs s ON s.k = c.k AND s.src = c.src AND s.stream = c.stream AND s.open_ms <= c.pub_ms
                       AND c.pub_ms < s.close_ms
        WHERE c.judged AND c.late),
    cv_kid_holds AS (
        SELECT x.k, x.src, x.node, x.seq,
               arg_min({'how': x.how, 'peer': x.peer, 'released_ms': x.released_ms}, x.t_ms) AS h
        FROM cv_arr x JOIN cv_kids USING (k, src, node, seq) GROUP BY ALL),
    cv_kid AS (
        SELECT d.k, d.src, d.seq, count(*) AS children, bool_or(h.h.released_ms IS NOT NULL) AS released,
               bool_or(h.h.how = 'push' AND h.h.peer = r.seed_node) AS pushed, bool_and(h.h IS NULL) AS never
        FROM cv_kids d LEFT JOIN cv_kid_holds h USING (k, src, node, seq) JOIN cv_r r ON r.k = d.k AND r.src = d.src
        GROUP BY ALL),
    cv_ended AS (
        SELECT k, src, stream, close_ms, end_reason FROM cv_subs WHERE end_reason IS NOT NULL)
    SELECT c.* EXCLUDE (parented),
           CASE WHEN c.judged AND c.late AND c.parented THEN coalesce(d.children, 0) END AS children,
           CASE WHEN NOT (c.judged AND c.late AND c.parented) THEN NULL WHEN d.children IS NULL THEN 'no_seed_child'
                WHEN d.released THEN 'released' WHEN d.pushed THEN 'pushed_late'
                WHEN d.never THEN 'child_never' ELSE 'other' END AS class,
           CASE WHEN c.judged AND c.late AND c.parented AND d.children IS NULL THEN e.end_reason END AS after
    FROM cv_chunks_of c LEFT JOIN cv_kid d USING (k, src, seq)
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
-- seed_children_in's, ms exact: 1000 less it is uncovered) and child_ms
-- (the seed's children's time in it, summed); both NULL for a run with
-- no parents rows (no push trees: no seed children to count).
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
