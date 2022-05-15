// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import qs from "qs";
import { useMemo } from "react";

const useQuery = <T>(queryString: string): T =>
  (useMemo(() => qs.parse(queryString, { ignoreQueryPrefix: true }) || {}, [
    queryString,
  ]) as unknown) as T;

export default useQuery;
