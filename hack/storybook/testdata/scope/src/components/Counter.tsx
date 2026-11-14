// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useState } from "react";

const Counter: React.FC = () => {
  const [n, setN] = useState(0);
  useEffect(() => {
    const worker = new Worker(new URL("./counter.worker.ts", import.meta.url));
    worker.onmessage = (e: MessageEvent<number>) => setN(e.data);
    return () => worker.terminate();
  }, []);
  return <span>{n}</span>;
};

export default Counter;
