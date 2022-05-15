// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { FundingClient } from "../../apis/client";
import create from "../../contexts/Api";

export const { ClientContext, Provider, useClient, useCall, useLazyCall } = create<FundingClient>();
