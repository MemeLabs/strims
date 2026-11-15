// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";
import { readdirSync } from "fs";
import { join } from "path";

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import { ErrorCode as RpcErrorCode } from "../apis/strims/rpc/errors/errors.pb";
import { errorCode, errorCodeEnums, errorCodeName, formatError } from "./rpcError";

const timeout = new RPCError({ code: RpcErrorCode.TIMEOUT, message: "call timed out" });

describe("rpcError", () => {
  it("reads the code of an rpc error only", () => {
    assert.strictEqual(errorCode(timeout), 1013);
    assert.strictEqual(errorCode(undefined), 0);
    assert.strictEqual(errorCode(new Error("x")), 0);
    assert.strictEqual(errorCode({ code: 20, message: "aborted" }), 0);
  });

  it("names a code in its API's enum", () => {
    assert.strictEqual(errorCodeName(1013), "TIMEOUT");
    assert.strictEqual(errorCodeName(0), undefined);
    assert.strictEqual(errorCodeName(987654), undefined);
  });

  it("formats the message with the code", () => {
    assert.strictEqual(formatError(timeout), "call timed out (1013 TIMEOUT)");
    assert.strictEqual(formatError(new RPCError({ message: "boom" })), "boom");
    assert.strictEqual(formatError(new RPCError({ code: 987654, message: "new" })), "new (987654)");
    assert.strictEqual(formatError(new Error("plain")), "plain");
    assert.strictEqual(formatError(undefined), undefined);
    assert.strictEqual(formatError(new RPCError()), undefined);
  });

  it("registers every generated error-code enum, whose codes are disjoint", async () => {
    const apis = join(__dirname, "../apis");
    const files = (readdirSync(apis, { recursive: true }) as string[]).filter((f) =>
      /(^|\/)errors\/errors\.pb\.ts$/.test(f)
    );
    assert.ok(files.length >= 4);
    for (const f of files) {
      const { ErrorCode } = (await import(join(apis, f))) as { ErrorCode: Record<number, string> };
      assert.ok(errorCodeEnums.includes(ErrorCode), `${f} is not in errorCodeEnums`);
    }
    const seen = new Map<number, number>();
    errorCodeEnums.forEach((codes, i) => {
      // A TS enum maps names to codes and codes back to names.
      for (const v of Object.values(codes as Record<string, string | number>)) {
        if (typeof v === "number" && v !== 0) {
          assert.ok(!seen.has(v), `code ${v} is in enums ${seen.get(v) ?? -1} and ${i}`);
          seen.set(v, i);
        }
      }
    });
  });
});
