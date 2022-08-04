// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

const first = <T>(it: Iterable<T>): T | undefined => {
  for (const v of it) {
    return v;
  }
};

export default first;
