---
tags: [sim-metrics]
max_turns: 40
timeout_seconds: 900
allowed_tools: [Read, Glob, Grep, Bash]
---

I want a `simdb.py coverage-report` command: seed coverage for an experiment's cells and arms. For each chunk the seed publishes, the time from its publishing to the first non-seed node holding it (from the arrivals table), with p50/p90/p99 by the verdict's phase, and the share never held. Plan it with me first: what you'd add or change, where, and the checks. No code yet.
