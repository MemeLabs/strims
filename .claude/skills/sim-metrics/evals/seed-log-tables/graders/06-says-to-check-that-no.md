---
type: llm
focus: trace
weight: 1
---

The answer says to check that no farm run is live (pgrep for farm.py/suite.py, or `farm.py status`) before landing the store_host change, because the farm hosts run store_host.py.
