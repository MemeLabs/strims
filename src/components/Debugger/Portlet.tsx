// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, {
  MouseEvent,
  MouseEventHandler,
  ReactNode,
  useCallback,
  useRef,
  useState,
} from "react";
import ReactDOM from "react-dom";
import { FiXSquare } from "react-icons/fi";
import { useClickAway } from "react-use";

const SizeContext = React.createContext<Size>({
  height: 0,
  width: 0,
});

export const usePortletSize = (): Size => React.useContext(SizeContext);

interface Position {
  top: number;
  left: number;
}

interface Size {
  height: number;
  width: number;
}

type ResizeDirection = "nw" | "n" | "ne" | "w" | "e" | "sw" | "s" | "se";

interface Pos {
  dragging: boolean;
  resizing: boolean;
  resizeDirection?: ResizeDirection;
  dragStart: Position;
  startPosition?: Position;
  position: Position;
  startSize?: Size;
  size: Size;
}

const DEFAULT_POS: Pos = {
  dragging: false,
  resizing: false,
  dragStart: {
    top: 0,
    left: 0,
  },
  position: {
    top: 0,
    left: 0,
  },
  size: {
    height: 400,
    width: 400,
  },
};

export interface PortletProps {
  onClose: () => void;
  isOpen: boolean;
  headerSize?: number;
  handleSize?: number;
  children: ReactNode;
}

const Portlet: React.FC<PortletProps> = ({
  onClose,
  isOpen,
  children,
  headerSize = 20,
  handleSize = 8,
}) => {
  const [pos, setPos] = useState(DEFAULT_POS);

  const handleDragStart = useCallback<MouseEventHandler>((e) => {
    setPos((pos) => ({
      ...pos,
      dragging: true,
      startPosition: pos.position,
      dragStart: {
        top: e.clientY,
        left: e.clientX,
      },
    }));
    e.preventDefault();
    e.stopPropagation();
  }, []);
  const handleDragEnd = useCallback(
    () => setPos((pos) => ({ ...pos, dragging: false, resizing: false })),
    []
  );

  const handleDrag = useCallback<MouseEventHandler>((e) => {
    setPos((pos) => {
      const dy = e.clientY - pos.dragStart.top;
      const dx = e.clientX - pos.dragStart.left;
      if (pos.dragging) {
        return {
          ...pos,
          position: {
            top: pos.startPosition.top + dy,
            left: pos.startPosition.left + dx,
          },
        };
      }
      if (pos.resizing) {
        switch (pos.resizeDirection) {
          case "nw":
            return {
              ...pos,
              position: {
                top: pos.startPosition.top + dy,
                left: pos.startPosition.left + dx,
              },
              size: {
                height: pos.startSize.height - dy,
                width: pos.startSize.width - dx,
              },
            };
          case "n":
            return {
              ...pos,
              position: {
                top: pos.startPosition.top + dy,
                left: pos.startPosition.left,
              },
              size: {
                height: pos.startSize.height - dy,
                width: pos.startSize.width,
              },
            };
          case "ne":
            return {
              ...pos,
              position: {
                top: pos.startPosition.top + dy,
                left: pos.startPosition.left,
              },
              size: {
                height: pos.startSize.height - dy,
                width: pos.startSize.width + dx,
              },
            };
          case "w":
            return {
              ...pos,
              position: {
                top: pos.startPosition.top,
                left: pos.startPosition.left + dx,
              },
              size: {
                height: pos.startSize.height,
                width: pos.startSize.width - dx,
              },
            };
          case "e":
            return {
              ...pos,
              size: {
                height: pos.startSize.height,
                width: pos.startSize.width + dx,
              },
            };
          case "sw":
            return {
              ...pos,
              position: {
                top: pos.startPosition.top,
                left: pos.startPosition.left + dx,
              },
              size: {
                height: pos.startSize.height + dy,
                width: pos.startSize.width - dx,
              },
            };
          case "s":
            return {
              ...pos,
              size: {
                height: pos.startSize.height + dy,
                width: pos.startSize.width,
              },
            };
          case "se":
            return {
              ...pos,
              size: {
                height: pos.startSize.height + dy,
                width: pos.startSize.width + dx,
              },
            };
        }
      }
      return pos;
    });
  }, []);

  const handleResizeStart = useCallback((e: MouseEvent, direction: ResizeDirection) => {
    setPos((pos) => ({
      ...pos,
      resizing: true,
      resizeDirection: direction,
      dragStart: {
        top: e.clientY,
        left: e.clientX,
      },
      startPosition: pos.position,
      startSize: pos.size,
    }));
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const dragContainer = useRef(null);

  useClickAway(dragContainer, () => setPos((pos) => ({ ...pos, dragging: false })));

  const dragContainerStyle: React.CSSProperties =
    pos.dragging || pos.resizing
      ? {
          position: "fixed",
          top: 0,
          left: 0,
          bottom: 0,
          right: 0,
          zIndex: 100,
        }
      : {};

  if (!isOpen) {
    return null;
  }

  const container = (
    <div
      ref={dragContainer}
      style={dragContainerStyle}
      onMouseUp={handleDragEnd}
      onMouseMove={handleDrag}
    >
      <div
        style={{
          position: "absolute",
          display: "flex",
          flexDirection: "column",
          zIndex: 100,
          ...pos.position,
          ...pos.size,
        }}
      >
        <div
          style={{
            flex: `0 0 ${handleSize}px`,
            display: "flex",
            flexDirection: "row",
          }}
        >
          <div
            style={{
              flex: `0 0 ${handleSize}px`,
              cursor: "nw-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "nw")}
          ></div>
          <div
            style={{
              flex: "1",
              cursor: "n-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "n")}
          ></div>
          <div
            style={{
              flex: `0 0 ${handleSize}px`,
              cursor: "ne-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "ne")}
          ></div>
        </div>
        <div
          style={{
            flex: "1",
            display: "flex",
            flexDirection: "row",
          }}
        >
          <div
            style={{
              flex: `0 0 ${handleSize}px`,
              cursor: "w-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "w")}
          ></div>
          <div
            style={{
              flex: "1",
              background: "white",
              flexDirection: "column",
              display: "flex",
            }}
          >
            <div
              style={{
                flex: `0 0 ${headerSize}px`,
                background: "orange",
                display: "flex",
                justifyContent: "flex-end",
                alignItems: "center",
                padding: "2px",
              }}
              onMouseDown={handleDragStart}
            >
              <span style={{ cursor: "pointer" }} onClick={onClose}>
                <FiXSquare />
              </span>
            </div>
            <div
              style={{
                flex: "1 0 0",
                overflow: "auto",
              }}
            >
              <div>
                <SizeContext.Provider
                  value={{
                    height: pos.size.height - handleSize * 2 - headerSize,
                    width: pos.size.width - handleSize * 2,
                  }}
                >
                  {children}
                </SizeContext.Provider>
              </div>
            </div>
          </div>
          <div
            style={{
              flex: `0 0 ${handleSize}px`,
              cursor: "e-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "e")}
          ></div>
        </div>
        <div
          style={{
            flex: `0 0 ${handleSize}px`,
            display: "flex",
            flexDirection: "row",
          }}
        >
          <div
            style={{
              flex: `0 0 ${handleSize}px`,
              cursor: "sw-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "sw")}
          ></div>
          <div
            style={{
              flex: "1",
              cursor: "s-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "s")}
          ></div>
          <div
            style={{
              flex: `0 0 ${handleSize}px`,
              cursor: "se-resize",
            }}
            onMouseDown={(e) => handleResizeStart(e, "se")}
          ></div>
        </div>
      </div>
    </div>
  );

  return ReactDOM.createPortal(container, document.body);
};

export default Portlet;
