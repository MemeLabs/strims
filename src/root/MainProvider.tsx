// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { Suspense } from "react";

import Layout from "../components/Layout";
import LoadingPlaceholder from "../components/LoadingPlaceholder";
import { Provider as ChatProvider } from "../contexts/Chat";
import { Provider as NetworkProvider } from "../contexts/Network";
import { Provider as NotificationProvider } from "../contexts/Notification";
import { Provider as PlayerProvider } from "../contexts/Player";

const MainProvider: React.FC = ({ children }) => (
  <NetworkProvider>
    <NotificationProvider>
      <ChatProvider>
        <PlayerProvider>
          <Layout>
            <Suspense fallback={<LoadingPlaceholder />}>{children}</Suspense>
          </Layout>
        </PlayerProvider>
      </ChatProvider>
    </NotificationProvider>
  </NetworkProvider>
);

export default MainProvider;
