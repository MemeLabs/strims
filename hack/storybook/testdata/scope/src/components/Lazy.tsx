// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { Suspense, lazy } from "react";

const Deep = lazy(() => import("./Deep"));

const Lazy: React.FC = () => (
  <Suspense fallback="loading">
    <Deep />
  </Suspense>
);

export default Lazy;
