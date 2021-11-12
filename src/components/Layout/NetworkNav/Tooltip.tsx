import "./NetworkNav.scss";

import React, { RefObject, useContext, useMemo, useRef, useState } from "react";
import usePortal from "use-portal";

import { useLayout } from "../../../contexts/Layout";

export interface TooltipOverlayProps {
  anchor: RefObject<HTMLElement>;
}

const TooltipOverlay: React.FC<TooltipOverlayProps> = ({ anchor, children }) => {
  const { root } = useLayout();
  const { Portal } = usePortal({ target: root.current });

  const rect = useMemo(() => anchor.current.getBoundingClientRect(), []);

  return (
    <Portal>
      <div
        style={{
          "--tooltip-anchor-x": `${rect.x}px`,
          "--tooltip-anchor-y": `${rect.y}px`,
          "--tooltip-anchor-width": `${rect.width}px`,
          "--tooltip-anchor-height": `${rect.height}px`,
        }}
        className="network_nav__tooltip__overlay"
      >
        {children}
      </div>
    </Portal>
  );
};

interface TooltipProps {
  label: string;
  visible?: boolean;
}

const Tooltip: React.FC<TooltipProps> = ({ children, label, visible = true }) => {
  const ref = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);

  return (
    <div
      className="network_nav__tooltip"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onWheel={() => setOpen(false)}
      onClick={() => setOpen(false)}
      ref={ref}
    >
      {children}
      {open && visible && <TooltipOverlay anchor={ref}>{label}</TooltipOverlay>}
    </div>
  );
};

export default Tooltip;
