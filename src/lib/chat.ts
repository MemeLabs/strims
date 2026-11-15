// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Message_DirectoryRef } from "../apis/strims/chat/v1/chat.pb";
import { getListingColor } from "./directory";

export const getDirectoryRefColor = (directoryRef: Message_DirectoryRef) =>
  directoryRef
    ? directoryRef.themeColor
      ? `#${directoryRef.themeColor.toString(16)}`
      : getListingColor(directoryRef.listing)
    : "";
