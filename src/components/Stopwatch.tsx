// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useState } from "react";

interface StopwatchProps {
  startTime: number;
}

const Stopwatch: React.FC<StopwatchProps> = ({ startTime }) => {
  const [now, setNow] = useState(Date.now() / 1000);

  useEffect(() => {
    let id = setTimeout(() => {
      id = setInterval(() => setNow(Date.now() / 1000), 1000);
    }, 1000 - (Date.now() % 1000));

    return () => {
      clearTimeout(id);
      clearInterval(id);
    };
  }, []);

  let d = now - startTime;
  const h = Math.floor(d / 3600);
  d %= 3600;
  const m = Math.floor(d / 60);
  d %= 60;
  const s = Math.floor(d);

  return <>{`${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`}</>;
};

export default Stopwatch;
