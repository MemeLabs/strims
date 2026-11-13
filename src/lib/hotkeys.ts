// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Options } from "react-hotkeys-hook";

// The layout's hotkeys, which work from inputs and the chat composer too:
// react-hotkeys-hook counts the composer's Slate editable (role="textbox")
// as a form field, checked before enableOnContentEditable.
export const globalHotkeyOptions: Options = {
  enableOnContentEditable: true,
  enableOnFormTags: ["INPUT", "textbox"],
};
