---
tags: [sim-metrics]
max_turns: 40
timeout_seconds: 900
allowed_tools: [Read, Glob, Grep, Bash]
---

The seed-cover branches' sims now write two new tables with SIM_SPEC / STRIMS_SIM_SPEC on: seed_decisions (a row per seed allocation decision: t_ms, kind, peer, ...) and seed_sends (a row per chunk the seed sent: peer, seq, how, queued_ms, ...). A few thousand rows a run. I want them in the sim lab so the coverage work can query them from the catalog. Don't change anything yet: give me the plan. Every file and registry to touch, on which branch and in what order, and the checks before it lands.
