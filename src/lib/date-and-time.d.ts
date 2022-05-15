// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

declare module "date-and-time" {
  const date: {
    format(dateObj: Date, formatString: string | string[], utc?: boolean): string;
  };
  export default date;
}
