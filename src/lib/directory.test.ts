// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import { ErrorCode } from "../apis/strims/network/v1/directory/errors/errors.pb";
import { ErrorCode as RpcErrorCode } from "../apis/strims/rpc/errors/errors.pb";
import { isAlreadyLeft } from "./directory";

const refusal = (code: number, message: string) => new RPCError({ code, message });

describe("isAlreadyLeft", () => {
  it("reads a gone listing, session or role as already left", () => {
    for (const [code, message] of [
      [ErrorCode.LISTING_NOT_FOUND, "listing not found"],
      [ErrorCode.SESSION_NOT_FOUND, "session not found"],
      [ErrorCode.NOT_VIEWING, "not viewing this listing"],
      [ErrorCode.NOT_PUBLISHING, "not publishing this listing"],
    ] as const) {
      assert.ok(isAlreadyLeft(refusal(code, message)), message);
    }
  });

  it("leaves other failures to surface", () => {
    assert.ok(!isAlreadyLeft(refusal(ErrorCode.VIEW_QUOTA, "exceeded concurrent view quota")));
    assert.ok(!isAlreadyLeft(refusal(RpcErrorCode.TIMEOUT, "call timed out")));
    // A message alone decides nothing.
    assert.ok(!isAlreadyLeft(refusal(0, "listing not found")));
    assert.ok(!isAlreadyLeft(undefined));
    assert.ok(!isAlreadyLeft(new Error("listing not found")));
  });
});
