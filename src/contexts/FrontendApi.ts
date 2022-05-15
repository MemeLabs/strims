// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { FrontendClient } from "../apis/client";
import create from "./Api";

export const {
  ClientContext,
  Provider,
  useClient,
  useCall,
  useLazyCall,
} = create<FrontendClient>();
