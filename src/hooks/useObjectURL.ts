// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useMemo } from "react";

const useObjectURL = (type: string, data: Uint8Array): string => {
  const url = useMemo(() => URL.createObjectURL(new Blob([data], { type })), [type, data]);
  useEffect(() => () => URL.revokeObjectURL(url), [url]);
  return url;
};

export default useObjectURL;
