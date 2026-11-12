// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { ReactNode, useCallback, useEffect, useMemo } from "react";
import { createPortal } from "react-dom";

interface UsePortalOptions {
  /** The element the portal's div is appended to; the document's body without one. */
  target?: HTMLElement | null;
}

interface PortalProps {
  children?: ReactNode;
}

// A div of the component's own, in `target` while it's mounted, and a
// Portal that renders its children into it.
const usePortal = ({ target }: UsePortalOptions = {}) => {
  const container = useMemo(() => document.createElement("div"), []);

  useEffect(() => {
    (target || document.body).appendChild(container);
    return () => container.remove();
  }, [target]);

  const Portal = useCallback(
    ({ children }: PortalProps) => createPortal(children, container),
    [container]
  );

  return { Portal };
};

export default usePortal;
