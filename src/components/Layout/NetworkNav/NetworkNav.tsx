import "./NetworkNav.scss";

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { ComponentProps, useContext } from "react";
import { DragDropContext, Draggable, DropResult, Droppable } from "react-beautiful-dnd";
import Scrollbars from "react-custom-scrollbars-2";
import { Trans, useTranslation } from "react-i18next";
import { BiNetworkChart } from "react-icons/bi";
import { BsArrowBarLeft, BsArrowBarRight } from "react-icons/bs";
import { FiPlus } from "react-icons/fi";
import { Link } from "react-router-dom";

import { Network } from "../../../apis/strims/network/v1/network";
import { useLayout } from "../../../contexts/Layout";
import { NetworkContext } from "../../../contexts/Network";
import { certificateRoot } from "../../../lib/certificate";
import NetworkAddButton from "./NetworkAddButton";
import NetworkGem from "./NetworkGem";
import Tooltip from "./Tooltip";

const ScrollbarView: React.FC<ComponentProps<"div">> = ({ style, ...props }) => (
  <div
    className="network_nav__list_content"
    {...props}
    style={{ ...style, overflowX: "hidden", marginBottom: 0 }}
  />
);

export interface NetworkNavItem {
  network: Network;
  peerCount: number;
}

const NetworkNav: React.FC = () => {
  const { t } = useTranslation();
  const { expandNav, toggleExpandNav } = useLayout();
  const [networks, { updateDisplayOrder }] = useContext(NetworkContext);

  const onDragEnd = React.useCallback((result: DropResult) => {
    if (result.destination) {
      updateDisplayOrder(result.source.index, result.destination.index);
    }
  }, []);

  return (
    <aside
      className={clsx({
        "network_nav": true,
        "network_nav--expanded": expandNav,
        "network_nav--collapsed": !expandNav,
      })}
    >
      <Tooltip label={expandNav ? t("layout.networkNav.Collapse") : t("layout.networkNav.Expand")}>
        <div className="network_nav__toggle">
          <div className="network_nav__toggle__text">
            <Trans>layout.networkNav.Networks</Trans>
          </div>

          <button onClick={() => toggleExpandNav()} className="network_nav__toggle__icon">
            {expandNav ? <BsArrowBarLeft /> : <BsArrowBarRight />}
          </button>
        </div>
      </Tooltip>
      <Tooltip label="Networks">
        <div className="network_nav__header_icon">
          <BiNetworkChart />
        </div>
      </Tooltip>
      <div className="network_nav__list">
        <Scrollbars autoHide renderView={ScrollbarView} style={{ overflowX: "hidden" }}>
          <DragDropContext onDragEnd={onDragEnd}>
            <Droppable droppableId="networks">
              {({ innerRef, droppableProps, placeholder }) => (
                <div ref={innerRef} {...droppableProps}>
                  {networks.map(({ network, peerCount }, i) => (
                    <Draggable
                      draggableId={`network-${network.id.toString()}`}
                      index={i}
                      key={network.id.toString()}
                    >
                      {({ innerRef, draggableProps, dragHandleProps }, { isDragging }) => {
                        const certRoot = certificateRoot(network.certificate);
                        return (
                          <div
                            className={clsx({
                              "network_nav__item": true,
                              "network_nav__item--dragging": isDragging,
                            })}
                            ref={innerRef}
                            {...draggableProps}
                            {...dragHandleProps}
                          >
                            <Tooltip label={certRoot.subject} visible={!expandNav && !isDragging}>
                              <Link
                                to={`/directory/${Base64.fromUint8Array(certRoot.key, true)}`}
                                className="network_nav__link"
                              >
                                <NetworkGem network={network} peerCount={peerCount} />
                                <div className="network_nav__link__text">{certRoot.subject}</div>
                              </Link>
                            </Tooltip>
                          </div>
                        );
                      }}
                    </Draggable>
                  ))}
                  {placeholder}
                </div>
              )}
            </Droppable>
          </DragDropContext>
          <Tooltip label={t("layout.networkNav.Add network")} visible={!expandNav}>
            <NetworkAddButton className="network_nav__add">
              <div className="network_nav__add__gem">
                <FiPlus />
              </div>
              <div className="network_nav__add__text">
                <Trans>layout.networkNav.Add network</Trans>
              </div>
            </NetworkAddButton>
          </Tooltip>
        </Scrollbars>
      </div>
    </aside>
  );
};

export default NetworkNav;
