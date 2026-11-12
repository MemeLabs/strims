// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { RefCallback, RefObject } from "react";

export type Ref<T> = RefCallback<T> | RefObject<T>;

export const setRef = <T>(ref: Ref<T>, v: T) => {
  if (ref instanceof Function) {
    ref(v);
  } else if (ref) {
    ref.current = v;
  }
};
