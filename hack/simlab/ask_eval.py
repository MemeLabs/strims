#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The console's Ask eval: the questions of ask_eval.json, asked of Claude
over the catalog (simlab.ask) and their answers scored against the
references, with what each cost.

  python3 hack/simlab/ask_eval.py check [--only ID,...]
      runs each question's reference SQL over the catalog and checks that
      every text and number its checks look for is among the values (free:
      no API call)
  python3 hack/simlab/ask_eval.py run [--only ID,...] [--budget N] [--jobs N] [--out FILE]
      asks each question in a conversation of its own (live API calls, priced
      into the spend ledger, target/console/spend.jsonl) and scores its answer:
      a check passes when the answer names its text, gives a number within its
      tolerance, or says one of its phrases; a question passes when all of its
      checks do. Prints each question's checks, cost, turns and time, and the
      totals, and writes them as JSON (target/console/ask-eval/ by default)

`run` needs the API's credentials where it runs (ANTHROPIC_API_KEY). Its
fetch_data calls fetch from S3 as the console's do, and its farm-run
proposals are quotes that nothing here approves.
"""

import argparse
import concurrent.futures as cf
import datetime
import json
import math
import os
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import ask, catalog, store_host  # noqa: E402

QUESTIONS = HERE / "ask_eval.json"
# A ratio's x (x1.016), and a name holding digits (star-15-10-end-abrupt,
# s7ae7e, lag_p99_ms, 439ms's unit), which aren't numbers of the answer.
RATIO = re.compile(r"(?<![\w.])[x×](?=\d)")
NAME = re.compile(r"[A-Za-z_]\w*(?:-\w+)*")
NUMBER = re.compile(r"\d+(?:,\d{3})*(?:\.\d+)?|\.\d+")


def load(only=None):
    qs = json.loads(QUESTIONS.read_text())["questions"]
    if only:
        missing = set(only) - {q["id"] for q in qs}
        if missing:
            sys.exit(f"ask_eval: no question {', '.join(sorted(missing))}")
        qs = [q for q in qs if q["id"] in only]
    return qs


def numbers(text):
    """The numbers `text` gives (their magnitudes: no check takes a
    negative one), thousands' commas read, the digits in names left out."""
    return [float(m.replace(",", "")) for m in NUMBER.findall(NAME.sub(" ", RATIO.sub(" ", text)))]


def near(x, want, tol):
    return abs(x - want) <= tol * abs(want) if want else x == 0


def passes(check, answer):
    """Whether `answer` (its text) passes `check`."""
    low = answer.lower()
    if "text" in check:
        return check["text"].lower() in low
    if "number" in check:
        return any(near(x, check["number"], check.get("tol", 0.02)) for x in numbers(answer))
    if "any" in check:
        return any(p.lower() in low for p in check["any"])
    if "none" in check:
        return not any(p.lower() in low for p in check["none"])
    raise ValueError(f"not a check: {check}")


def label(check):
    if "text" in check:
        return repr(check["text"])
    if "number" in check:
        return f"{check['number']:g}" + (f" ±{check['tol']:.0%}" if check.get("tol") else "")
    if "any" in check:
        return "one of " + " / ".join(repr(p) for p in check["any"][:3]) + (" …" if len(check["any"]) > 3 else "")
    return "none of " + " / ".join(repr(p) for p in check["none"])


def flat(v):
    if isinstance(v, (list, tuple)):
        for x in v:
            yield from flat(x)
    elif isinstance(v, dict):
        for x in v.values():
            yield from flat(x)
    else:
        yield v


def pool(size=2):
    return catalog.pool("ask-eval", size)


def check(args):
    """Every question's checks found among its reference SQL's values."""
    p = pool()
    bad = 0
    try:
        for q in load(args.only):
            values = []
            for sql in q["reference_sql"]:
                values += list(flat(p.query(sql, max_rows=10_000, timeout_s=300).values()))
            texts = [v.lower() for v in values if isinstance(v, str)]
            nums = [float(v) for v in values if isinstance(v, (int, float)) and not isinstance(v, bool)
                    and math.isfinite(v)]
            missing = []
            for c in q["checks"]:
                if "text" in c and not any(c["text"].lower() == t or c["text"].lower() in t for t in texts):
                    missing.append(label(c))
                elif "number" in c and not any(near(x, c["number"], c.get("tol", 0.02)) for x in nums):
                    missing.append(label(c))
            bad += bool(missing)
            print(f"{q['id']:24} {'ok' if not missing else 'MISSING ' + ', '.join(missing)}")
    finally:
        p.close()
    print(f"\n{len(load(args.only)) - bad} of {len(load(args.only))} references hold")
    return 1 if bad else 0


def run(args):
    """Each question asked, its answer scored and priced."""
    if not os.environ.get("ANTHROPIC_API_KEY"):
        sys.exit("ask_eval run: no ANTHROPIC_API_KEY: export the API key (the eval's calls are billed)")
    qs = load(args.only)
    p = pool(size=max(2, args.jobs * 2))
    asker = ask.Asker(p, lambda: catalog.schema(p))
    print(f"ask_eval: {len(qs)} questions, {ask.MODEL} at {ask.EFFORT} effort, fallbacks {ask.FALLBACKS}, "
          f"task budget {args.budget:,} tokens, {args.jobs} at a time; spend ledger {asker.ledger.path}",
          file=sys.stderr)

    def one(q):
        t0 = time.monotonic()

        def emit(e):
            if e["type"] == "tool":
                purpose = e["input"].get("purpose") if isinstance(e["input"], dict) else None
                print(f"  {q['id']}: {e['name']}" + (f" ({purpose})" if purpose else ""), file=sys.stderr)
            elif e["type"] in ("error", "refusal"):
                print(f"  {q['id']}: {e['type']}: {e.get('message') or e.get('category')}", file=sys.stderr)
        try:
            a = asker.ask(q["question"], task_budget=args.budget, emit=emit)
            answer, stop, usd, turns, error = a.text, a.stop_reason, a.usd, a.turns, a.error
        except Exception as e:
            answer, stop, usd, turns, error = "", "error", 0.0, 0, f"{type(e).__name__}: {e}"
        results = [{"check": label(c), "passed": passes(c, answer)} for c in q["checks"]]
        return {"id": q["id"], "question": q["question"], "passed": all(r["passed"] for r in results) and not error,
                "checks": results, "usd": usd, "turns": turns, "stop_reason": stop, "error": error,
                "elapsed_s": round(time.monotonic() - t0, 1), "answer": answer, "reference": q["answer"]}

    try:
        with cf.ThreadPoolExecutor(max_workers=args.jobs) as ex:
            got = list(ex.map(one, qs))
    finally:
        p.close()
    for r in got:
        failed = [c["check"] for c in r["checks"] if not c["passed"]]
        print(f"{r['id']:24} {'PASS' if r['passed'] else 'FAIL'}  {sum(c['passed'] for c in r['checks'])}/"
              f"{len(r['checks'])} checks  ${r['usd']:.4f}  {r['turns']} turns  {r['elapsed_s']:.0f} s"
              + (f"  stopped: {r['stop_reason']}" if r["stop_reason"] != "end_turn" else "")
              + (f"\n{'':26}missing {', '.join(failed)}" if failed else "")
              + (f"\n{'':26}{r['error']}" if r["error"] else ""))
    total = sum(r["usd"] for r in got)
    passed = sum(r["passed"] for r in got)
    print(f"\n{passed} of {len(got)} passed; ${total:.4f} in all, ${total / max(1, len(got)):.4f} a question")
    out = Path(args.out) if args.out else (store_host.main_checkout() / "target" / "console" / "ask-eval"
                                           / f"{datetime.datetime.now():%Y%m%dT%H%M%S}.json")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps({"model": ask.MODEL, "effort": ask.EFFORT, "task_budget": args.budget,
                               "passed": passed, "questions": len(got), "usd": round(total, 6), "results": got},
                              indent=1, ensure_ascii=False) + "\n")
    print(f"results: {out}", file=sys.stderr)
    return 0 if passed == len(got) else 1


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    for name in ("check", "run"):
        s = sub.add_parser(name)
        s.add_argument("--only", type=lambda v: [x for x in v.split(",") if x], help="question ids, comma-separated")
        if name == "run":
            s.add_argument("--budget", type=int, default=ask.TASK_BUDGET, help="each question's task budget (tokens)")
            s.add_argument("--jobs", type=int, default=4, help="questions asked at once")
            s.add_argument("--out", help="the results' JSON file")
    a = p.parse_args(argv)
    if a.cmd == "run" and not ask.MIN_TASK_BUDGET <= a.budget <= ask.MAX_TASK_BUDGET:
        p.error(f"--budget: {ask.MIN_TASK_BUDGET:,} to {ask.MAX_TASK_BUDGET:,}")
    sys.exit(check(a) if a.cmd == "check" else run(a))


if __name__ == "__main__":
    main()
