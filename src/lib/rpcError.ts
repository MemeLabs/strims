// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import { ErrorCode as ChatErrorCode } from "../apis/strims/chat/v1/errors/errors";
import { ErrorCode as DaoErrorCode } from "../apis/strims/dao/v1/errors/errors";
import { ErrorCode as DirectoryErrorCode } from "../apis/strims/network/v1/directory/errors/errors";
import { ErrorCode as NetworkErrorCode } from "../apis/strims/network/v1/errors/errors";
import { ErrorCode as NodeErrorCode } from "../apis/strims/node/errors/errors";
import { ErrorCode as RpcErrorCode } from "../apis/strims/rpc/errors/errors";

// Every API's error-code enum. Each codes its errors in a range of its own
// (strims-proto's build/errors.rs), so a code is in at most one of them.
export const errorCodeEnums: readonly Record<number, string>[] = [
  NetworkErrorCode,
  RpcErrorCode,
  DaoErrorCode,
  NodeErrorCode,
  DirectoryErrorCode,
  ChatErrorCode,
];

// The code of `e`, a call's rejection: its rpc.Error's code, or 0 (no
// code) for anything else, such as the undefined a timed-out call rejects
// with or a DOMException's legacy code.
export const errorCode = (e: unknown): number => (e instanceof RPCError ? e.code : 0);

// The name of `code` in its API's enum, if it has one.
export const errorCodeName = (code: number): string | undefined => {
  if (code === 0) {
    return undefined;
  }
  for (const codes of errorCodeEnums) {
    const name = codes[code];
    if (typeof name === "string") {
      return name;
    }
  }
  return undefined;
};

// `e`'s message for display, followed by its code when it has one, e.g.
// "listing not found (4000 LISTING_NOT_FOUND)". Undefined when `e` has no
// message.
export const formatError = (e: unknown): string | undefined => {
  const message = (e as { message?: unknown } | null | undefined)?.message;
  if (typeof message !== "string" || message === "") {
    return undefined;
  }
  const code = errorCode(e);
  if (code === 0) {
    return message;
  }
  const name = errorCodeName(code);
  return `${message} (${name === undefined ? code : `${code} ${name}`})`;
};
