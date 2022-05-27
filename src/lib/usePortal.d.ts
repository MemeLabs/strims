// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "react-useportal";

declare module "react-useportal" {
  interface UsePortalResult {
    isOpen: () => void;
    openPortal: () => void;
    togglePortal: () => void;
    closePortal: () => void;
    Portal: React.ElementType;
  }

  export default function usePortal({
    closeOnOutsideClick,
    closeOnEsc,
    bindTo,
    isOpen: defaultIsOpen,
    onOpen,
    onClose,
    onPortalClick,
    ...eventHandlers
  }?: UsePortalOptions): UsePortalResult;
}
