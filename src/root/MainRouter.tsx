// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactElement, lazy } from "react";
import { Navigate, Route, Routes } from "react-router-dom";

import LayoutBody from "../components/Layout/Body";
import { useBackgroundRoute } from "../contexts/BackgroundRoute";
import Invite from "../pages/Invite";
import NotFound from "../pages/NotFound";
import SettingsLayout from "../pages/Settings/Layout";

const Network = lazy(() => import(/* webpackPrefetch: true */ "../pages/Settings/Network"));
const Bootstrap = lazy(() => import(/* webpackPrefetch: true */ "../pages/Settings/Bootstrap"));
const Chat = lazy(() => import(/* webpackPrefetch: true */ "../pages/Settings/Chat"));
const Video = lazy(() => import(/* webpackPrefetch: true */ "../pages/Settings/Video"));
const VNIC = lazy(() => import(/* webpackPrefetch: true */ "../pages/Settings/VNIC"));
const Autoseed = lazy(() => import(/* webpackPrefetch: true */ "../pages/Settings/Autoseed"));

const Broadcast = lazy(() => import(/* webpackPrefetch: true */ "../pages/Broadcast"));
const Categories = lazy(() => import(/* webpackPrefetch: true */ "../pages/Categories"));
const Directory = lazy(() => import(/* webpackPrefetch: true */ "../pages/Directory"));
const Embed = lazy(() => import(/* webpackPrefetch: true */ "../pages/Embed"));
const Home = lazy(() => import(/* webpackPrefetch: true */ "../pages/Home"));
const Player = lazy(() => import(/* webpackPrefetch: true */ "../pages/Player"));
const Streams = lazy(() => import(/* webpackPrefetch: true */ "../pages/Streams"));

export const createSettingsRoutes = (layout: ReactElement) => (
  <Route path="settings/*" element={layout}>
    <Route index element={<Navigate replace to="networks" />} />
    <Route path="networks/*" element={<Network />} />
    <Route path="bootstraps/*" element={<Bootstrap />} />
    <Route path="chat-servers/*" element={<Chat />} />
    <Route path="video/*" element={<Video />} />
    <Route path="vnic/*" element={<VNIC />} />
    <Route path="autoseed/*" element={<Autoseed />} />
  </Route>
);

const settingsRoutes = createSettingsRoutes(<SettingsLayout />);

const mainRoutes = (
  <Route path="*" element={<LayoutBody />}>
    <Route index element={<Home />} />
    <Route path="invite/:code" element={<Invite />} />
    <Route path="directory/:networkKey" element={<Directory />} />
    <Route path="player/:networkKey" element={<Player />} />
    <Route path="embed/:service/:id" element={<Embed />} />
    <Route path="categories" element={<Categories />} />
    <Route path="broadcast" element={<Broadcast />} />
    <Route path="streams" element={<Streams />} />
    <Route path="*" element={<NotFound />} />
  </Route>
);

const MainRouter: React.FC = () => {
  const { location } = useBackgroundRoute();
  return (
    <Routes location={location}>
      {settingsRoutes}
      {mainRoutes}
    </Routes>
  );
};

export default MainRouter;
