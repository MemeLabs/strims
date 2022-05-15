// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Transform, TransformCallback } from "stream";

export class AsyncPassThrough extends Transform {
  _transform(chunk: any, encoding: BufferEncoding, callback: TransformCallback): void {
    setTimeout(() => callback(null, chunk));
  }
}
