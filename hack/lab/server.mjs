// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Device lab server.
//
// - http://LAN:8080    the dev CA certificate and links (plain HTTP, so a
//                      phone can fetch the CA before it trusts the HTTPS site)
// - https://LAN:8443   the agent page, its files, and the control WebSocket
//                      at /control
// - http://127.0.0.1:8090  the controller API, loopback only:
//     GET  /clients                  connected agents
//     POST /eval?client=ID&timeout=MS  body is an async function body run
//                                    on the agent (default: newest client)
//     GET  /log?n=N                  the last N events
//
// usage: node server.mjs LAN_IP

import fs from "node:fs";
import http from "node:http";
import https from "node:https";
import path from "node:path";
import { createRequire } from "node:module";

const require = createRequire("/home/slugalisk/projects/strims/go-ppspp/package.json");
const WebSocket = require("ws");

const HOST = process.argv[2];
const DIR = path.dirname(new URL(import.meta.url).pathname);
const WWW = path.join(DIR, "www");
const EVENTS = path.join(DIR, "events.jsonl");
const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mp4": "video/mp4",
  ".h264": "application/octet-stream",
  ".crt": "application/x-x509-ca-cert",
  ".json": "application/json",
};

const events = [];
function record(ev) {
  ev.at = new Date().toISOString();
  events.push(ev);
  if (events.length > 2000) events.shift();
  fs.appendFileSync(EVENTS, JSON.stringify(ev) + "\n");
}

function serveFile(req, res) {
  const url = new URL(req.url, "https://x");
  const rel = decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname);
  const file = path.resolve(WWW, "." + rel);
  if (!file.startsWith(WWW + path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
    res.writeHead(404).end("not found");
    return;
  }
  // Range requests get 206: iOS plays a media URL only from a server that
  // answers them.
  const size = fs.statSync(file).size;
  const headers = {
    "Content-Type": TYPES[path.extname(file)] || "application/octet-stream",
    "Cache-Control": "no-store",
    "Accept-Ranges": "bytes",
  };
  const m = /^bytes=(\d*)-(\d*)$/.exec(req.headers.range || "");
  if (m && (m[1] || m[2])) {
    let start = m[1] ? Number(m[1]) : Math.max(0, size - Number(m[2]));
    let end = m[1] && m[2] ? Math.min(Number(m[2]), size - 1) : size - 1;
    if (start > end || start >= size) {
      res.writeHead(416, { "Content-Range": `bytes */${size}` }).end();
      return;
    }
    res.writeHead(206, { ...headers, "Content-Range": `bytes ${start}-${end}/${size}`, "Content-Length": end - start + 1 });
    fs.createReadStream(file, { start, end }).pipe(res);
    return;
  }
  res.writeHead(200, { ...headers, "Content-Length": size });
  fs.createReadStream(file).pipe(res);
}

// Plain HTTP: the CA, and a page pointing at the HTTPS site.
http
  .createServer((req, res) => {
    if (req.url === "/strims-dev-ca.crt") {
      res.writeHead(200, { "Content-Type": "application/x-x509-ca-cert", "Cache-Control": "no-store" });
      return res.end(fs.readFileSync(path.join(DIR, "tls/ca.crt")));
    }
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" }).end(
      `<!doctype html><meta name="viewport" content="width=device-width, initial-scale=1">` +
        `<title>Strims device lab</title><body style="font:18px -apple-system,system-ui,sans-serif;padding:16px">` +
        `<h1 style="font-size:22px">Strims device lab</h1>` +
        `<p>1. <a href="/strims-dev-ca.crt">Download the dev CA</a>, install it, and turn on full trust for it.</p>` +
        `<p>2. Open <a href="https://${HOST}:8443/">https://${HOST}:8443/</a></p>`,
    );
  })
  .listen(8080, HOST);

// HTTPS: the agent page and the control socket.
const tls = {
  key: fs.readFileSync(path.join(DIR, "tls/server.key")),
  cert: fs.readFileSync(path.join(DIR, "tls/server.crt")),
};
const site = https.createServer(tls, serveFile).listen(8443, HOST);

const clients = new Map();
const pending = new Map();
let nextClient = 1;
let nextMsg = 1;

new WebSocket.Server({ server: site, path: "/control" }).on("connection", (ws, req) => {
  const id = nextClient++;
  const c = { id, ws, addr: req.socket.remoteAddress, connectedAt: Date.now(), lastSeen: Date.now(), hello: null };
  clients.set(id, c);
  record({ client: id, type: "connect", addr: c.addr });
  ws.on("message", (data) => {
    c.lastSeen = Date.now();
    let msg;
    try {
      msg = JSON.parse(data);
    } catch (e) {
      return;
    }
    if (msg.type === "hello") {
      c.hello = msg;
      c.unlocked = !!msg.unlocked;
      c.awake = !!msg.awake;
    }
    if (msg.type === "event" && typeof msg.event === "string") {
      if (msg.event.startsWith("unlocked")) c.unlocked = true;
      if (msg.event === "wake lock held") c.awake = true;
      if (msg.event === "wake lock released") c.awake = false;
      if (msg.event === "visibility hidden") c.visible = false;
      if (msg.event === "visibility visible") c.visible = true;
    }
    if (msg.type === "result" && pending.has(msg.id)) {
      const p = pending.get(msg.id);
      pending.delete(msg.id);
      clearTimeout(p.timer);
      p.resolve(msg);
    }
    record({ client: id, ...msg });
  });
  ws.on("pong", () => (c.lastSeen = Date.now()));
  ws.on("close", () => {
    clients.delete(id);
    record({ client: id, type: "disconnect" });
  });
});
setInterval(() => clients.forEach((c) => c.ws.ping()), 15000);

// Controller API, loopback only.
http
  .createServer(async (req, res) => {
    const url = new URL(req.url, "http://x");
    const reply = (code, v) => res.writeHead(code, { "Content-Type": "application/json" }).end(JSON.stringify(v, null, 1));
    if (url.pathname === "/clients") {
      return reply(
        200,
        [...clients.values()].map((c) => ({
          id: c.id,
          addr: c.addr,
          ua: c.hello && c.hello.ua,
          secure: c.hello && c.hello.secure,
          loads: c.hello && c.hello.loads,
          unlocked: !!c.unlocked,
          awake: !!c.awake,
          visible: c.visible !== false,
          connectedSecs: Math.round((Date.now() - c.connectedAt) / 1000),
          lastSeenSecs: Math.round((Date.now() - c.lastSeen) / 1000),
        })),
      );
    }
    if (url.pathname === "/log") return reply(200, events.slice(-(+url.searchParams.get("n") || 50)));
    if (url.pathname === "/eval" && req.method === "POST") {
      let code = "";
      for await (const chunk of req) code += chunk;
      const want = url.searchParams.get("client");
      const c = want ? clients.get(+want) : [...clients.values()].pop();
      if (!c) return reply(404, { error: "no such client" });
      const id = nextMsg++;
      const timeout = +url.searchParams.get("timeout") || 60000;
      const result = await new Promise((resolve) => {
        pending.set(id, { resolve, timer: setTimeout(() => (pending.delete(id), resolve({ ok: false, error: "timeout" })), timeout) });
        c.ws.send(JSON.stringify({ op: "eval", id, code }));
      });
      return reply(200, { client: c.id, ...result });
    }
    reply(404, { error: "unknown" });
  })
  .listen(8090, "127.0.0.1");

console.log(`lab up: http://${HOST}:8080/  https://${HOST}:8443/  controller http://127.0.0.1:8090/`);
