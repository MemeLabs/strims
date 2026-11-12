---
tags: [sim-metrics]
max_turns: 40
timeout_seconds: 900
allowed_tools: [Read, Glob, Grep, Bash]
---

Add a row field to both sims: the share of the seed's pushes that went to a peer that already held the chunk (a duplicate push), and score it in the suite so a scorecard shows it. Lay out what you'd change in each sim and in the lab, and the checks it needs before landing. Plan only, no edits.
