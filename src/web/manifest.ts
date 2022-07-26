// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import twemoji from "../../assets/chat/TwemojiMozilla.ttf";
import { wasmPath } from "./svc.go";

export default [wasmPath, ...MANIFEST, twemoji];
