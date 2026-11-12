---
type: llm
focus: trace
weight: 1
---

The answer puts the computation in a new `hack/simlab` module (such as `coverage.py`) whose module docstring pins the definition, reading the catalog through `simlab.engine`, not the cache's files.
