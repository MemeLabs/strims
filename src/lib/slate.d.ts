// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { BaseEditor, BaseRange, BaseText } from "slate";
import { HistoryEditor } from "slate-history";
import { ReactEditor } from "slate-react";

type EntityToken = {
  code?: boolean;
  spoiler?: boolean;
  url?: boolean;
  emote?: boolean;
  tag?: boolean;
  nick?: boolean;
  self?: boolean;
  greentext?: boolean;
};

type Text = BaseText & EntityToken;

declare module "slate" {
  interface CustomTypes {
    Editor: BaseEditor & ReactEditor & HistoryEditor;
    Element: { type: "paragraph"; children: Text[] };
    Text: Text;
    Range: BaseRange & EntityToken;
  }
}
