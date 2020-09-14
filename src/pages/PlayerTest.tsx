import { Base64 } from "js-base64";
import * as React from "react";
import { useParams } from "react-router-dom";

import { MainLayout } from "../components/MainLayout";
import VideoPlayer from "../components/VideoPlayer";
import { useClient } from "../contexts/Api";

interface PlayerTestRouteParams {
  networkKey: string;
  swarmKey: string;
}

const PlayerTest = () => {
  const params = useParams<PlayerTestRouteParams>();
  const [loaded, setLoaded] = React.useState(false);
  const client = useClient();

  React.useEffect(() => {
    client.startVPN();
    setTimeout(() => setLoaded(true), 1000);
  }, []);

  return (
    <MainLayout>
      <main className="home_page__main">
        <header className="home_page__subheader"></header>
        <section className="home_page__main__video" style={{ position: "relative" }}>
          {loaded && (
            <VideoPlayer
              networkKey={Base64.toUint8Array(params.networkKey)}
              swarmKey={Base64.toUint8Array(params.swarmKey)}
              mimeType="video/fmp4"
            />
          )}
        </section>
      </main>
      <aside className="home_page__right">
        <header className="home_page__subheader"></header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat chat"></div>
      </aside>
    </MainLayout>
  );
};

export default PlayerTest;
