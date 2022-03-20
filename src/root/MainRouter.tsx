import React, { ReactElement, lazy } from "react";
import { Navigate, Route, Routes } from "react-router-dom";

import LayoutBody from "../components/Layout/Body";
import { useBackgroundRoute } from "../contexts/BackgroundRoute";
import SettingsLayout from "../pages/Settings/Layout";

const Network = lazy(() => import("../pages/Settings/Network"));
const BootstrapClients = lazy(() => import("../pages/Settings/BootstrapClients"));
const Chat = lazy(() => import("../pages/Settings/Chat"));
const Video = lazy(() => import("../pages/Settings/Video"));
const VNIC = lazy(() => import("../pages/Settings/VNIC"));
const Autoseed = lazy(() => import("../pages/Settings/Autoseed"));

const Broadcast = lazy(() => import("../pages/Broadcast"));
const Categories = lazy(() => import("../pages/Categories"));
const Directory = lazy(() => import("../pages/Directory"));
const Embed = lazy(() => import("../pages/Embed"));
const Home = lazy(() => import("../pages/Home"));
const PlayerTest = lazy(() => import("../pages/PlayerTest"));
const Streams = lazy(() => import("../pages/Streams"));

export const createSettingsRoutes = (layout: ReactElement) => (
  <Route path="settings/*" element={layout}>
    <Route index element={<Navigate replace to="networks" />} />
    <Route path="networks/*" element={<Network />} />
    <Route path="bootstrap-clients/*" element={<BootstrapClients />} />
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
    <Route path="directory/:networkKey" element={<Directory />} />
    <Route path="player/:networkKey" element={<PlayerTest />} />
    <Route path="embed/:service/:id" element={<Embed />} />
    <Route path="categories" element={<Categories />} />
    <Route path="broadcast" element={<Broadcast />} />
    <Route path="streams" element={<Streams />} />
    <Route path="*" element={<Navigate to="/404" />} />
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
