// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";

import { Metrics } from "../../apis/strims/debug/v1/metrics";
import { useClient } from "../../contexts/FrontendApi";

export const HISTORY_LENGTH = 240;
export const SAMPLE_INTERVAL_MS = 1000;
// The time the history spans once it's full.
export const HISTORY_SECONDS = (HISTORY_LENGTH * SAMPLE_INTERVAL_MS) / 1000;

export interface MetricsHistory {
  // The latest snapshots, oldest first.
  snapshots: Metrics[];
  error?: string;
}

// Watches the node's metrics while mounted.
const useMetricsHistory = (): MetricsHistory => {
  const client = useClient();
  const [history, setHistory] = useState<MetricsHistory>({ snapshots: [] });

  useEffect(() => {
    const events = client.debug.watchMetrics({ intervalMs: SAMPLE_INTERVAL_MS });
    events.on("data", ({ metrics }) => {
      if (metrics) {
        setHistory(({ snapshots }) => ({
          snapshots: [...snapshots.slice(1 - HISTORY_LENGTH), metrics],
        }));
      }
    });
    events.on("error", ({ message }) => setHistory((prev) => ({ ...prev, error: message })));
    return () => events.destroy();
  }, []);

  return history;
};

export default useMetricsHistory;
