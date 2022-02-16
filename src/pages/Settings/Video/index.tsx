import React from "react";
import { Route, Routes } from "react-router-dom";

import VideoChannelCreateForm from "./VideoChannelCreateForm";
import VideoChannelEditForm from "./VideoChannelEditForm";
import VideoChannelsList from "./VideoChannelsList";
import VideoIngressConfigForm from "./VideoIngressConfigForm";

const Router: React.FC = () => {
  return (
    <main className="network_page">
      <Routes>
        <Route index element={<VideoIngressConfigForm />} />
        <Route path="channels" element={<VideoChannelsList />} />
        <Route path="channels/new" element={<VideoChannelCreateForm />} />
        <Route path="channels/:channelId" element={<VideoChannelEditForm />} />
      </Routes>
    </main>
  );
};

export default Router;