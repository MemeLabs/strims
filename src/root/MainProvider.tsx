import React, { Suspense } from "react";

import Layout from "../components/Layout";
import { Provider as ChatProvider } from "../contexts/Chat";
import { Provider as DirectoryProvider } from "../contexts/Directory";
import { Provider as NetworkProvider } from "../contexts/Network";
import { Provider as NotificationProvider } from "../contexts/Notification";
import { Provider as PlayerProvider } from "../contexts/Player";
import LoadingPlaceholder from "./LoadingPlaceholder";

const MainProvider: React.FC = ({ children }) => (
  <DirectoryProvider>
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
  </DirectoryProvider>
);

export default MainProvider;
