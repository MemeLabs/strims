// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "react-useportal";

import React from "react";

declare module "react-useportal" {
  interface UsePortalResult {
    isOpen: () => void;
    openPortal: (e: React.MouseEvent) => void;
    togglePortal: (open: boolean) => void;
    closePortal: () => void;
    Portal: React.ElementType;
  }

  export default function usePortal(options?: UsePortalOptions): UsePortalResult;
}
