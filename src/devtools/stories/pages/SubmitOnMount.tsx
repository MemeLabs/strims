// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useEffect, useRef } from "react";

// Submits the first form under it once, as it mounts, so a form's own
// validation shows its errors.
export const SubmitOnMount: React.FC<{ children: ReactNode }> = ({ children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => ref.current?.querySelector("form")?.requestSubmit(), []);
  return (
    <div ref={ref} style={{ display: "contents" }}>
      {children}
    </div>
  );
};
