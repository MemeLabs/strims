import clsx from "clsx";
import { Base64 } from "js-base64";
import Tooltip from "rc-tooltip";
import * as React from "react";
import { ReactElement } from "react";
import {
  DragDropContext,
  Draggable,
  DropResult,
  Droppable,
  ResponderProvided,
} from "react-beautiful-dnd";
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
import { Link, useHistory } from "react-router-dom";
import { useToggle } from "react-use";
import usePortal from "react-useportal";

import { useCall, useClient } from "../contexts/Api";
import { useTheme } from "../contexts/Theme";
import useObjectURL from "../hooks/useObjectURL";
import { CreateNetworkResponse, ICertificate, INetwork } from "../lib/pb";
import AddNetworkModal from "./AddNetworkModal";

const NetworkAddButton: React.FunctionComponent<React.ComponentProps<"button">> = ({
  children,
  ...props
}) => {
  const { isOpen, openPortal, closePortal, Portal } = usePortal();
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

const rootCertificate = (cert: ICertificate): ICertificate =>
  cert.parent ? rootCertificate(cert.parent) : cert;

interface NetworkGemProps {
  network: INetwork;
}

const NetworkGem: React.FC<NetworkGemProps> = ({ network }) => {
  const { icon } = network;
  const gemClassName = clsx({
    "main_layout__left__link__gem": true,
    "main_layout__left__link__gem--with_icon": !!icon,
    "main_layout__left__link__gem--without_icon": !icon,
  });

  if (icon) {
    const url = useObjectURL(icon.type, icon.data);
    return <div className={gemClassName} style={{ backgroundImage: `url(${url})` }} />;
  }

  const backgroundColor = "green";
  return (
    <div className={gemClassName} style={{ backgroundColor }}>
      {network.name.substr(0, 1)}
    </div>
  );
};

const NetworkNav = () => {
  const [expanded, toggleExpanded] = useToggle(false);
  const [networks, setNetworks] = React.useState<INetwork[]>([]);
  const [state, { setNavOrder }] = useTheme();

  const [{ error, loading }] = useCall("network", "list", {
    onComplete: (res) => setNetworks(res.networks),
  });

  const onDragEnd = React.useCallback((result: DropResult, provided: ResponderProvided) => {
    if (!result.destination) {
      return;
    }

    setNetworks((prev) => {
      const next = Array.from(prev);
      const [target] = next.splice(result.source.index, 1);
      next.splice(result.destination.index, 0, target);
      setNavOrder(next.map(({ id }) => id));
      return next;
    });
  }, []);

  let links: ReactElement;
  if (error) {
    links = <div>error</div>;
  } else if (loading) {
    links = <div>...</div>;
  } else {
    links = (
      <>
        <Tooltip placement="right" overlay="Networks">
          <div className="main_layout__left__header_icon">
            <BiNetworkChart />
          </div>
        </Tooltip>
        <DragDropContext onDragEnd={onDragEnd}>
          <Droppable droppableId="networks">
            {(provided, snapshot) => (
              <div ref={provided.innerRef} {...provided.droppableProps}>
                {networks.map((network, i) => (
                  <Draggable draggableId={`network-${network.id}`} index={i} key={network.id}>
                    {(provided, snapshot) => (
                      <div
                        ref={provided.innerRef}
                        {...provided.draggableProps}
                        {...provided.dragHandleProps}
                      >
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
                            <NetworkGem network={network} />
                            <div className="main_layout__left__link__text">
                              <span>{network.name}</span>
                            </div>
                          </Link>
                        </Tooltip>
                      </div>
                    )}
                  </Draggable>
                ))}
                {provided.placeholder}
              </div>
            )}
          </Droppable>
        </DragDropContext>
      </>
    );
  }

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

export const MainLayout = ({ children }: { children: any }) => {
  const [theme, { setColorScheme }] = useTheme();

  const toggleTheme = () =>
    theme.colorScheme === "dark" ? setColorScheme("light") : setColorScheme("dark");

  return (
    <div className="main_layout">
      <header className="main_layout__header">
        <div className="main_layout__primary_nav">
          <button onClick={toggleTheme} className="main_layout__primary_nav__logo">
            <FiHome />
          </button>
          <Link to="/settings" className="main_layout__primary_nav__link">
            Categories
          </Link>
          <Link to="/" className="main_layout__primary_nav__link">
            Streams
          </Link>
          <Link to="/broadcast" className="main_layout__primary_nav__link">
            Broadcast
          </Link>
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
          <Link to="/alerts" className="main_layout__user_nav__link">
            <FiBell />
          </Link>
          <Link to="/" className="main_layout__user_nav__link">
            <FiCloud />
          </Link>
          <Link to="/profile" className="main_layout__user_nav__link">
            <FiUser />
          </Link>
        </div>
      </header>
      <div className="main_layout__content">
        <NetworkNav />
        {children}
      </div>
    </div>
  );
};
