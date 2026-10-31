// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The device agent: a long-lived page that dials the lab's control socket,
// runs the code it is sent, and forwards its console and errors. It never
// navigates, so the user activation from one tap lasts for the session.

(function () {
  const statusEl = document.getElementById("status");
  const logEl = document.getElementById("log");
  document.getElementById("secure").textContent = String(window.isSecureContext);
  document.getElementById("secure").className = window.isSecureContext ? "ok" : "bad";

  let loads = 0;
  try {
    loads = +(sessionStorage.getItem("lab-loads") || 0) + 1;
    sessionStorage.setItem("lab-loads", String(loads));
  } catch (e) {}

  let ws = null;
  const queue = [];
  function send(msg) {
    const s = JSON.stringify(msg);
    if (ws && ws.readyState === 1) ws.send(s);
    else if (queue.length < 500) queue.push(s);
  }

  function show(line) {
    logEl.textContent = (line + "\n" + logEl.textContent).slice(0, 8000);
  }

  function safe(v) {
    try {
      return JSON.parse(
        JSON.stringify(v, (k, x) => (x instanceof Error ? x.name + ": " + x.message : x))
      );
    } catch (e) {
      return String(v);
    }
  }

  for (const level of ["log", "info", "warn", "error"]) {
    const orig = console[level].bind(console);
    console[level] = (...args) => {
      orig(...args);
      send({ type: "log", level, args: args.map(safe), t: Date.now() });
    };
  }
  window.addEventListener("error", (e) =>
    send({ type: "error", message: e.message, source: e.filename + ":" + e.lineno, t: Date.now() })
  );
  window.addEventListener("unhandledrejection", (e) =>
    send({ type: "error", message: "unhandled rejection: " + safe(e.reason), t: Date.now() })
  );
  document.addEventListener("visibilitychange", () =>
    send({ type: "event", event: "visibility " + document.visibilityState, t: Date.now() })
  );
  window.addEventListener("pagehide", () =>
    send({ type: "event", event: "pagehide", t: Date.now() })
  );

  const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

  async function run(msg) {
    show("run #" + msg.id);
    try {
      const value = await new AsyncFunction(msg.code)();
      send({ type: "result", id: msg.id, ok: true, value: safe(value) });
    } catch (e) {
      send({
        type: "result",
        id: msg.id,
        ok: false,
        error: e && e.name ? e.name + ": " + e.message : String(e),
      });
    }
  }

  function connect() {
    ws = new WebSocket(
      (location.protocol === "https:" ? "wss://" : "ws://") + location.host + "/control"
    );
    ws.onopen = () => {
      statusEl.textContent = "connected";
      statusEl.className = "ok";
      send({
        type: "hello",
        ua: navigator.userAgent,
        secure: window.isSecureContext,
        loads,
        unlocked: !!window.labUnlocked,
        awake: !!(window.labWake && !window.labWake.released),
        screen: [screen.width, screen.height, devicePixelRatio],
        t: Date.now(),
      });
      while (queue.length) ws.send(queue.shift());
    };
    ws.onmessage = (m) => {
      let msg;
      try {
        msg = JSON.parse(m.data);
      } catch (e) {
        return;
      }
      if (msg.op === "eval") run(msg);
    };
    ws.onclose = () => {
      statusEl.textContent = "reconnecting…";
      statusEl.className = "bad";
      setTimeout(connect, 1000);
    };
  }
  connect();

  const wakeEl = document.getElementById("wake");

  // Safari grants a wake lock only within a tap's activation, so a tap
  // handler calls this before anything else awaits. The lock goes when
  // the page is hidden, and only another tap takes it back.
  function takeWake() {
    if (!navigator.wakeLock) {
      wakeEl.textContent = "No wake lock in this browser";
      send({ type: "error", message: "wake lock: no navigator.wakeLock", t: Date.now() });
      return;
    }
    navigator.wakeLock.request("screen").then(
      (w) => {
        window.labWake = w;
        wakeEl.textContent = "Screen kept awake";
        wakeEl.className = "held";
        w.addEventListener("release", () => {
          wakeEl.textContent = "Screen lock released: tap to keep awake";
          wakeEl.className = "";
          send({ type: "event", event: "wake lock released", t: Date.now() });
        });
        send({ type: "event", event: "wake lock held", t: Date.now() });
      },
      (e) => {
        wakeEl.textContent = "Wake lock failed (" + e.name + "): tap to retry";
        send({ type: "error", message: "wake lock: " + safe(e), t: Date.now() });
      }
    );
  }
  wakeEl.addEventListener("click", takeWake);

  // 0.1 s of silence as a WAV: the spare element plays it, unmuted, in the tap.
  function silentWav() {
    const rate = 8000,
      n = 800,
      b = new DataView(new ArrayBuffer(44 + n * 2));
    const str = (o, s) => [...s].forEach((c, i) => b.setUint8(o + i, c.charCodeAt(0)));
    str(0, "RIFF");
    b.setUint32(4, 36 + n * 2, true);
    str(8, "WAVEfmt ");
    b.setUint32(16, 16, true);
    b.setUint16(20, 1, true);
    b.setUint16(22, 1, true);
    b.setUint32(24, rate, true);
    b.setUint32(28, rate * 2, true);
    b.setUint16(32, 2, true);
    b.setUint16(34, 16, true);
    str(36, "data");
    b.setUint32(40, n * 2, true);
    return URL.createObjectURL(new Blob([b], { type: "audio/wav" }));
  }

  document.getElementById("unlock").addEventListener("click", async () => {
    if (!window.labWake || window.labWake.released) takeWake();
    // An element played unmuted within a tap may play unmuted later without
    // one (iOS): probes borrow window.labSpareVideo for sound, through
    // window.labSpareSource (its node in labAudio, connected to the output).
    const spare = window.labSpareVideo || document.createElement("video");
    spare.playsInline = true;
    spare.muted = false;
    spare.src = silentWav();
    const spared = spare.play().then(
      () => spare.pause(),
      (e) => send({ type: "error", message: "spare: " + safe(e), t: Date.now() })
    );
    try {
      const AC = window.AudioContext || window.webkitAudioContext;
      window.labAudio = new AC();
      await window.labAudio.resume();
      await spared;
      window.labSpareVideo = spare;
      window.labSpareSource = window.labAudio.createMediaElementSource(spare);
      window.labSpareSource.connect(window.labAudio.destination);
      const v = document.createElement("video");
      v.playsInline = true;
      v.muted = true;
      v.src = "av.mp4";
      v.controls = true;
      document.body.appendChild(v);
      await v.play().catch(() => {});
      window.labVideo = v;
      window.labUnlocked = true;
      document.getElementById("unlock").textContent =
        "Unlocked (audio " + window.labAudio.state + ")";
      send({ type: "event", event: "unlocked, audio " + window.labAudio.state, t: Date.now() });
    } catch (e) {
      send({ type: "error", message: "unlock: " + safe(e), t: Date.now() });
    }
  });
})();
