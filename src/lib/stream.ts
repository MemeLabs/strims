import { Transform, TransformCallback } from "stream";

export class AsyncPassThrough extends Transform {
  _transform(chunk: any, encoding: BufferEncoding, callback: TransformCallback): void {
    setTimeout(() => callback(null, chunk));
  }
}
