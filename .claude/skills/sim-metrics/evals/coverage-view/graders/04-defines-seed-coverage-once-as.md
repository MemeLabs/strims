---
type: llm
focus: trace
weight: 1
---

The answer defines seed coverage once, as a macro in `hack/simlab/sql/scoring.sql` that the view and the coverage-report module both call, not in the view's own SQL.
