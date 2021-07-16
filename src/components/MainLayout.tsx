import clsx from "clsx";
import { Base64 } from "js-base64";
// import Tooltip from "rc-tooltip";
import React, { useCallback, useState } from "react";
import { DragDropContext, Draggable, DropResult, Droppable } from "react-beautiful-dnd";
import { BiNetworkChart } from "react-icons/bi";
import {
  FiActivity,
  FiArrowLeft,
  FiArrowRight,
  FiBell,
  FiCloud,
  FiHome,
  FiPlus,
  FiSearch,
  FiUser,
} from "react-icons/fi";
import { Link, NavLink, useHistory } from "react-router-dom";
import { useToggle } from "react-use";
import usePortal from "react-useportal";

import { MetricsFormat } from "../apis/strims/debug/v1/debug";
import { CreateNetworkResponse, Network, NetworkEvent } from "../apis/strims/network/v1/network";
import { useClient } from "../contexts/FrontendApi";
import { useTheme } from "../contexts/Theme";
import useObjectURL from "../hooks/useObjectURL";
import { rootCertificate } from "../lib/certificate";
import AddNetworkModal from "./AddNetworkModal";
import Badge from "./Badge";
import Debugger from "./Debugger";

const Tooltip: React.FC<any> = ({ children }) => <>{children}</>;

const NetworkAddButton: React.FC<React.ComponentProps<"button">> = ({ children, ...props }) => {
  const { isOpen, openPortal, closePortal, Portal } = usePortal() as {
    isOpen: () => void;
    openPortal: () => void;
    closePortal: () => void;
    Portal: React.ElementType;
  };
  const history = useHistory();

  const handleCreate = (res: CreateNetworkResponse) => {
    history.push(`/directory/${Base64.fromUint8Array(res.network.key.public, true)}`);
    closePortal();
  };

  return (
    <>
      <button {...props} onClick={openPortal}>
        {children}
      </button>
      {isOpen && (
        <Portal>
          <AddNetworkModal onCreate={handleCreate} onClose={closePortal} />
        </Portal>
      )}
    </>
  );
};

type NetworkGemProps = NetworkNavItem;

const NetworkGem: React.FC<NetworkGemProps> = ({ network, peerCount }) => {
  const { icon } = network;
  const gemClassName = clsx({
    "main_layout__left__link__gem": true,
    "main_layout__left__link__gem--with_icon": !!icon,
    "main_layout__left__link__gem--without_icon": !icon,
  });

  if (icon) {
    const url = useObjectURL(icon.type, icon.data);
    return (
      <div className={gemClassName} style={{ backgroundImage: `url(${url})` }}>
        <Badge count={peerCount} />
      </div>
    );
  }

  const backgroundColor = "green";
  return (
    <div className={gemClassName} style={{ backgroundColor }}>
      {network.name.substr(0, 1)}
      <Badge count={peerCount} />
    </div>
  );
};

interface NetworkNavItem {
  network: Network;
  peerCount: number;
}

// TODO: move to theme context
const useNetworkStuff = () => {
  const client = useClient();
  const [items, setItems] = React.useState<NetworkNavItem[]>([]);

  const addItem = (item: NetworkNavItem) =>
    setItems((prev) => {
      const next = Array.from(prev);
      const i = next.findIndex(
        ({ network: { displayOrder } }) => displayOrder > item.network.displayOrder
      );
      next.splice(i === -1 ? next.length : i, 0, item);
      return next;
    });

  const removeItem = (networkId: bigint) =>
    setItems((prev) => prev.filter((item) => item.network.id !== networkId));

  const setPeerCount = (networkId: bigint, peerCount: number) =>
    setItems((prev) =>
      prev.map((item) => (item.network.id === networkId ? { ...item, peerCount: peerCount } : item))
    );

  const reorderNetworks = (srcIndex: number, dstIndex: number) =>
    setItems((prev) => {
      const next = Array.from(prev);
      const [target] = next.splice(srcIndex, 1);
      next.splice(dstIndex, 0, target);
      void client.network.setDisplayOrder({ networkIds: next.map(({ network: { id } }) => id) });
      return next;
    });

  React.useEffect(() => {
    const events = client.network.watch();
    events.on("data", ({ event: { body } }) => {
      switch (body.case) {
        case NetworkEvent.BodyCase.NETWORK_START:
          addItem(body.networkStart);
          break;
        case NetworkEvent.BodyCase.NETWORK_STOP:
          removeItem(body.networkStop.networkId);
          break;
        case NetworkEvent.BodyCase.NETWORK_PEER_COUNT_UPDATE:
          setPeerCount(
            body.networkPeerCountUpdate.networkId,
            body.networkPeerCountUpdate.peerCount
          );
          break;
      }
    });

    return () => events.destroy();
  }, []);

  const ops = { reorderNetworks };

  return [items, ops] as [NetworkNavItem[], typeof ops];
};

const NetworkNav: React.FC = () => {
  const [expanded, toggleExpanded] = useToggle(false);
  const [networks, { reorderNetworks }] = useNetworkStuff();
  // const [state, { setNavOrder }] = useTheme();

  const onDragEnd = React.useCallback((result: DropResult) => {
    if (result.destination) {
      reorderNetworks(result.source.index, result.destination.index);
    }
  }, []);

  const links = (
    <>
      <Tooltip placement="right" overlay="Networks">
        <div className="main_layout__left__header_icon">
          <BiNetworkChart />
        </div>
      </Tooltip>
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
                  {({ innerRef, draggableProps, dragHandleProps }) => (
                    <div ref={innerRef} {...draggableProps} {...dragHandleProps}>
                      <Tooltip
                        placement="right"
                        trigger={["hover"]}
                        overlay={network.name}
                        {...(expanded ? { visible: false } : {})}
                      >
                        <Link
                          to={`/directory/${Base64.fromUint8Array(
                            rootCertificate(network.certificate).key,
                            true
                          )}`}
                          className="main_layout__left__link"
                        >
                          <NetworkGem network={network} peerCount={peerCount} />
                          <div className="main_layout__left__link__text">
                            <span>{network.name}</span>
                          </div>
                        </Link>
                      </Tooltip>
                    </div>
                  )}
                </Draggable>
              ))}
              {placeholder}
            </div>
          )}
        </Droppable>
      </DragDropContext>
    </>
  );

  const classes = clsx({
    "main_layout__left": true,
    "main_layout__left--expanded": expanded,
    "main_layout__left--collapsed": !expanded,
  });

  return (
    <aside className={classes}>
      <div className="main_layout__left__toggle">
        <div className="main_layout__left__toggle__text">Networks</div>
        <Tooltip
          placement="right"
          trigger={["hover", "click"]}
          overlay={expanded ? "Collapse" : "Expand"}
        >
          <button onClick={toggleExpanded} className="main_layout__left__toggle__icon">
            {expanded ? <FiArrowLeft /> : <FiArrowRight />}
          </button>
        </Tooltip>
      </div>
      {links}
      <NetworkAddButton className="main_layout__left__add">
        <div className="main_layout__left__add__gem">
          <FiPlus />
        </div>
        <div className="main_layout__left__add__text">Add</div>
      </NetworkAddButton>
    </aside>
  );
};

export const MainLayout: React.FC = ({ children }) => {
  const [theme, { setColorScheme }] = useTheme();
  const client = useClient();

  const [debuggerIsOpen, setDebuggerIsOpen] = useState(false);
  const handleDebuggerClose = useCallback(() => setDebuggerIsOpen(false), []);
  const handleDebuggerOpen = () => setDebuggerIsOpen(true);

  const toggleTheme = () =>
    theme.colorScheme === "dark" ? setColorScheme("light") : setColorScheme("dark");

  const handleAlertsClick = async () => {
    const { data } = await client.debug.readMetrics({
      format: MetricsFormat.METRICS_FORMAT_TEXT,
    });
    console.log(new TextDecoder().decode(data));
  };

  return (
    <div className="main_layout">
      <header className="main_layout__header">
        <div className="main_layout__primary_nav">
          <button onClick={toggleTheme} className="main_layout__primary_nav__logo">
            <FiHome />
          </button>
          <NavLink
            to="/settings"
            className="main_layout__primary_nav__link"
            activeClassName="main_layout__primary_nav__link--active"
          >
            Categories
          </NavLink>
          <NavLink
            to="/"
            exact
            className="main_layout__primary_nav__link"
            activeClassName="main_layout__primary_nav__link--active"
          >
            Streams
          </NavLink>
          <NavLink
            to="/broadcast"
            className="main_layout__primary_nav__link"
            activeClassName="main_layout__primary_nav__link--active"
          >
            Broadcast
          </NavLink>
        </div>
        <div className="main_layout__search">
          <input className="main_layout__search__input" placeholder="search..." />
          <button className="main_layout__search__button">
            <FiSearch />
          </button>
        </div>
        <div className="main_layout__user_nav">
          <Link to="/activity" className="main_layout__user_nav__link">
            <FiActivity />
          </Link>
          <button onClick={handleAlertsClick} className="main_layout__user_nav__link">
            <FiBell />
          </button>
          <button onClick={handleDebuggerOpen} className="main_layout__user_nav__link">
            <FiCloud />
          </button>
          <Link to="/profile" className="main_layout__user_nav__link">
            <FiUser />
          </Link>
        </div>
      </header>
      <div className="main_layout__content">
        <NetworkNav />
        {children}
      </div>
      <Debugger isOpen={debuggerIsOpen} onClose={handleDebuggerClose} />
    </div>
  );
};
