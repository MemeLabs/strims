import "./styles/main.scss";

import React from "react";
import ReactDOM from "react-dom";

import { DevToolsClient } from "../apis/client";
import { WSReadWriter } from "../lib/ws";
import App from "./root/App";

// import registerServiceWorker, { ServiceWorkerNoSupportError } from "service-worker-loader!./sw";

// void registerServiceWorker({ scope: "/" })
//   .then((registration) => {
//     console.log(registration);
//   })
//   .catch((e) => {
//     if (e instanceof ServiceWorkerNoSupportError) {
//       console.log("service worker not supported");
//     } else {
//       console.log("some service worker bullshit", e);
//     }
//   });

// (() => {
//   console.log("installing beforeinstallprompt handler");
//   let deferredPrompt: unknown;
//   window.addEventListener("beforeinstallprompt", (e) => {
//     console.log("received beforeinstallprompt");
//     // Prevent Chrome 67 and earlier from automatically showing the prompt
//     e.preventDefault();
//     // Stash the event so it can be triggered later.
//     deferredPrompt = e;
//     // Update UI to notify the user they can add to home screen
//     // installButton.current.style.display = 'block';

//     // installButton.current.addEventListener("click", (e) => {
//     //   console.log("handling click...");
//     //   // hide our user interface that shows our A2HS button
//     //   // installButton.current.style.display = 'none';
//     //   // Show the prompt
//     //   deferredPrompt.prompt();
//     //   // Wait for the user to respond to the prompt
//     //   deferredPrompt.userChoice.then((choiceResult) => {
//     //     console.log("received userChoice", choiceResult.outcome);
//     //     if (choiceResult.outcome === "accepted") {
//     //       console.log("User accepted the A2HS prompt");
//     //     } else {
//     //       console.log("User dismissed the A2HS prompt");
//     //     }
//     //     deferredPrompt = null;
//     //   });
//     // });
//   });
// })();

(() => {
  const ws: any = new WSReadWriter(`wss://${location.host}/api`);
  const client = new DevToolsClient(ws, ws);

  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App client={client} />, root);
})();
