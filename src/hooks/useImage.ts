// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Image } from "../apis/strims/type/image";
import useObjectURL from "../hooks/useObjectURL";
import { toFileType } from "../lib/image";

export const useImage = (image: Image): string => useObjectURL(toFileType(image.type), image.data);
