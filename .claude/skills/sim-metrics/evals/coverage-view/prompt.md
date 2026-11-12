---
tags: [sim-metrics]
max_turns: 40
timeout_seconds: 900
allowed_tools: [Read, Glob, Grep, Bash]
---

Next to the coverage-report command we're adding, I want a console view that plots seed coverage (publish to first non-seed hold, from arrivals) per arm over stream time for an experiment and cell, with the verdict's phases as bands. Plan it: the files, where the metric's definition lives, and the checks. Don't edit anything.
