// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { MutableRefObject, RefCallback } from "react";

export type Ref<T> = RefCallback<T> | MutableRefObject<T>;

export const setRef = <T>(ref: Ref<T>, v: T) => {
  if (ref instanceof Function) {
    ref(v);
  } else if (ref) {
    ref.current = v;
  }
};
