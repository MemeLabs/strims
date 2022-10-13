// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

export const compareBigInts = (a: bigint, b: bigint): number => (a === b ? 0 : a < b ? -1 : 1);
