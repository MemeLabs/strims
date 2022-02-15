import React from "react";
import { Route, Routes } from "react-router-dom";

import VideoChannelsList from "./VideoChannelsList";
import VideoIngressConfigForm from "./VideoIngressConfigForm";

const Router: React.FC = () => {
  return (
    <main className="network_page">
      <Routes>
        <Route index element={<VideoIngressConfigForm />} />
        <Route path="channels" element={<VideoChannelsList />} />
      </Routes>
    </main>
  );
};

export default Router;
