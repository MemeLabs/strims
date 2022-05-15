// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

export const retrySync = (fn: () => boolean, delay: number, attempts: number) =>
  setTimeout(() => {
    if (!fn() && attempts > 0) retrySync(fn, delay * 2, attempts - 1);
  }, delay);
