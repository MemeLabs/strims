// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Network } from "../apis/strims/network/v1/network";
import { certificateRoot } from "./certificate";

export const networkKey = (network: Network): Uint8Array =>
  certificateRoot(network.certificate).key;
