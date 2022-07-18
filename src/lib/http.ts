// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Transform, TransformCallback } from "stream";

export class HTTPReadWriter extends Transform {
  constructor(private uri: string) {
    super();
  }

  _transform(chunk: Uint8Array, _: BufferEncoding, callback: TransformCallback): void {
    void fetch(this.uri, {
      method: "POST",
      headers: {
        "Content-Type": "application/protobuf",
      },
      body: chunk,
    })
      .then((res) => res.arrayBuffer())
      .then((data) => this.push(new Uint8Array(data)));

    callback();
  }
}
