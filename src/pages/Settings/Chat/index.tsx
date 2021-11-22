import React, { ComponentProps, createContext, useContext } from "react";
import { Location, Route, Routes, useHref, useLinkClickHandler, useRoutes } from "react-router-dom";

import ChatEmoteCreateForm from "./ChatEmoteCreateForm";
import ChatEmoteEditForm from "./ChatEmoteEditForm";
import ChatEmoteList from "./ChatEmoteList";
import ChatModifierCreateForm from "./ChatModifierCreateForm";
import ChatModifierEditForm from "./ChatModifierEditForm";
import ChatModifierList from "./ChatModifierList";
import ChatServerCreateForm from "./ChatServerCreateForm";
import ChatServerEditForm from "./ChatServerEditForm";
import ChatServerList from "./ChatServerList";
import ChatTagCreateForm from "./ChatTagCreateForm";
import ChatTagEditForm from "./ChatTagEditForm";
import ChatTagList from "./ChatTagList";

const Router: React.FC = () => (
  <main className="network_page">
    <Routes>
      <Route index element={<ChatServerList />} />
      <Route path="new" element={<ChatServerCreateForm />} />
      <Route path=":serverId" element={<ChatServerEditForm />} />
      <Route path=":serverId/emotes" element={<ChatEmoteList />} />
      <Route path=":serverId/emotes/new" element={<ChatEmoteCreateForm />} />
      <Route path=":serverId/emotes/:emoteId" element={<ChatEmoteEditForm />} />
      <Route path=":serverId/modifiers" element={<ChatModifierList />} />
      <Route path=":serverId/modifiers/new" element={<ChatModifierCreateForm />} />
      <Route path=":serverId/modifiers/:modifierId" element={<ChatModifierEditForm />} />
      <Route path=":serverId/tags" element={<ChatTagList />} />
      <Route path=":serverId/tags/new" element={<ChatTagCreateForm />} />
      <Route path=":serverId/tags/:tagId" element={<ChatTagEditForm />} />
    </Routes>
  </main>
);

interface NavigationPushOptions {
  back?: boolean;
}

interface NavigationContextValues {
  history: Location[];
  focusedIndex: number;
  push(location: Location, options?: NavigationPushOptions);
  focusPrev(): void;
  focusNext(): void;
}

const NavigationContext = createContext<NavigationContextValues>(null);

export const useNavigation = () => useContext(NavigationContext);
export const useInNavigationContext = () => useNavigation() !== undefined;
export const useLinkClickHandler2 = (to: string, options: NavigationPushOptions) => {
  const navigation = useNavigation();

  const location = {
    pathname: to,
    hash: "",
    search: "",
    state: null,
    key: null,
  };

  return () => void navigation.push(location, options);
};
export const useBackLinkClickHandler = () => {
  const navigation = useNavigation();
  return () => navigation.focusPrev();
};

interface LinkProps extends ComponentProps<"a"> {
  to: string;
  back?: boolean;
}

export const Link: React.FC<LinkProps> = ({ to, back, ...props }) => {
  const handleClick = useInNavigationContext()
    ? useLinkClickHandler2(to, { back })
    : useLinkClickHandler(useHref(to));

  return <a {...props} onClick={handleClick} />;
};

export const Something: React.FC = () => {
  return null;
};

export default Router;
