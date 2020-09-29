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
  FiPlusCircle,
  FiSearch,
  FiUser,
  FiVideo,
} from "react-icons/fi";
import { Link, useHistory } from "react-router-dom";
import { useToggle } from "react-use";
import usePortal from "react-useportal";

import { useCall, useClient } from "../contexts/Api";
import { useTheme } from "../contexts/Theme";
import { CreateNetworkResponse, INetworkMembership } from "../lib/pb";
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

const NetworkNav = () => {
  const [expanded, toggleExpanded] = useToggle(false);
  const [networkMemberships, setNetworkMemberships] = React.useState<INetworkMembership[]>([]);
  const [state, { setNavOrder }] = useTheme();

  const [{ error, loading }] = useCall("getNetworkMemberships", {
    onComplete: (res) => setNetworkMemberships(res.networkMemberships),
  });

  const onDragEnd = React.useCallback((result: DropResult, provided: ResponderProvided) => {
    if (!result.destination) {
      return;
    }

    setNetworkMemberships((prev) => {
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
                {networkMemberships.map((membership, i) => (
                  <Draggable draggableId={`network-${membership.id}`} index={i} key={membership.id}>
                    {(provided, snapshot) => (
                      <div
                        ref={provided.innerRef}
                        {...provided.draggableProps}
                        {...provided.dragHandleProps}
                      >
                        <Tooltip
                          placement="right"
                          trigger={["hover"]}
                          overlay={membership.name}
                          {...(expanded ? { visible: false } : {})}
                        >
                          <Link
                            to={`/directory/${Base64.fromUint8Array(
                              membership.caCertificate.key,
                              true
                            )}`}
                            className="main_layout__left__link"
                          >
                            <div className="main_layout__left__link__gem">
                              {membership.name.substr(0, 1)}
                            </div>
                            <div className="main_layout__left__link__text">
                              <span>{membership.name}</span>
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
          <Link to="/networks" className="main_layout__primary_nav__link">
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
