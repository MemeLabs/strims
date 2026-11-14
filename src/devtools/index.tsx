// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// First: in a storybook frame it makes the realm the frame's device's
// before any app module reads the user agent.
import "./storybook/host/preload";
import "./styles/main.scss";
import "../lib/i18n";

import { frameDocument } from "./storybook/host/preload";

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
  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  // A storybook frame renders its story (or the story index) alone, without
  // the devtools API. Each loads its own modules, so a frame never evaluates
  // the app's (hack/storybook/check.py's scope counts what a frame loads).
  if (frameDocument) {
    void import("./storybook/host/Frame").then(({ renderFrame }) =>
      renderFrame(root, frameDocument)
    );
    return;
  }

  void import("./root/render").then(({ renderApp }) => renderApp(root));
})();
