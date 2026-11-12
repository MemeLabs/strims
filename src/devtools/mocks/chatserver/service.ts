// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as chatv1 from "../../../apis/strims/chat/v1/chat";
import { Emote, Modifier, Server, ServerIcon, Tag } from "../../../apis/strims/chat/v1/chat";
import { UnimplementedChatServerFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import {
  ModifierRecord,
  emote,
  emoteFixtures,
  emoteImage,
  loadAssets,
  modifiers,
  server,
  serverIcon,
  tags,
} from "./fixtures";

export interface ChatServerServiceOptions {
  /** Fixture servers 1..servers (default 3), or these servers. */
  servers?: number | Server[];
  /** Each server's first `emotes` fixture emotes (default all of them). */
  emotes?: number;
  /** Each server's first `modifiers` fixture modifiers (default all of them). */
  modifiers?: number;
  /** Each server's first `tags` fixture tags (default all of them). */
  tags?: number;
  /** Servers that have no icon (default server 2). */
  iconless?: bigint[];
}

interface EmoteRecord {
  emote: Emote;
  /** The fixture image a meta-only emote loads, absent for created ones. */
  url?: string;
}

const errNotFound = (what: string) => new Error(`${what} not found`);

// One chat server host's servers, icons, emotes, modifiers and tags, in
// memory: what the create, update and delete calls change, later reads see.
// Fixture servers are in NetworkService's fixture networks 1..n; their
// emotes' images load from the chat mock's PNGs when a call wants them.
export default class ChatServerService extends UnimplementedChatServerFrontendService {
  servers: Server[];
  /** Icons by server id: null for none; a fixture server's loads on its first read. */
  icons = new Map<bigint, ServerIcon | null>();
  emotes = new Map<bigint, EmoteRecord>();
  modifiers = new Map<bigint, ModifierRecord>();
  tags = new Map<bigint, Tag>();
  nextId = BigInt(100000);

  constructor({
    servers = 3,
    emotes = emoteFixtures.length,
    modifiers: modifierCount = Infinity,
    tags: tagCount = Infinity,
    iconless = [BigInt(2)],
  }: ChatServerServiceOptions = {}) {
    super();
    this.servers =
      typeof servers === "number"
        ? Array.from({ length: servers }, (_, i) => server(i + 1))
        : servers;
    for (const { id } of this.servers) {
      const base = id * BigInt(10000);
      if (iconless.includes(id)) {
        this.icons.set(id, null);
      }
      emoteFixtures.slice(0, emotes).forEach((f, i) => {
        const emoteId = base + BigInt(i + 1);
        this.emotes.set(emoteId, { emote: emote(id, emoteId, f), url: f.url });
      });
      for (const m of modifiers(id, modifierCount, base + BigInt(5001))) {
        this.modifiers.set(m.modifier.id, m);
      }
      for (const t of tags(id, tagCount, base + BigInt(8001))) {
        this.tags.set(t.id, t);
      }
    }
  }

  private newId(): bigint {
    return this.nextId++;
  }

  private findServer(id: bigint): Server {
    const s = this.servers.find((s) => s.id === id);
    if (!s) {
      throw errNotFound(`server ${id}`);
    }
    return s;
  }

  private find<T>(map: Map<bigint, T>, what: string, id: bigint): T {
    const v = map.get(id);
    if (v === undefined) {
      throw errNotFound(`${what} ${id}`);
    }
    return v;
  }

  // The values of `map` whose serverId is `serverId`, in id order.
  private ofServer<T>(map: Map<bigint, T>, serverId: bigint, get: (v: T) => bigint): T[] {
    this.findServer(serverId);
    return [...map.values()].filter((v) => get(v) === serverId);
  }

  // The record's emote with its images.
  private async loadEmote(r: EmoteRecord): Promise<Emote> {
    if (r.emote.images.length || !r.url) {
      return r.emote;
    }
    r.emote = new Emote({ ...r.emote, images: [await emoteImage(r.url)] });
    return r.emote;
  }

  createServer(req: chatv1.CreateServerRequest): chatv1.CreateServerResponse {
    const id = this.newId();
    const s = new Server({ ...server(Number(id)), id, networkKey: req.networkKey, room: req.room });
    this.servers = [...this.servers, s];
    this.icons.set(s.id, null);
    return new chatv1.CreateServerResponse({ server: s });
  }

  updateServer(req: chatv1.UpdateServerRequest): chatv1.UpdateServerResponse {
    const s = new Server({
      ...this.findServer(req.id),
      networkKey: req.networkKey,
      room: req.room,
    });
    this.servers = this.servers.map((v) => (v.id === s.id ? s : v));
    return new chatv1.UpdateServerResponse({ server: s });
  }

  deleteServer(req: chatv1.DeleteServerRequest): chatv1.DeleteServerResponse {
    this.findServer(req.id);
    this.servers = this.servers.filter((s) => s.id !== req.id);
    this.icons.delete(req.id);
    const drop = <T>(map: Map<bigint, T>, serverId: (v: T) => bigint) => {
      for (const [id, v] of map) {
        if (serverId(v) === req.id) {
          map.delete(id);
        }
      }
    };
    drop(this.emotes, (r) => r.emote.serverId);
    drop(this.modifiers, (r) => r.modifier.serverId);
    drop(this.tags, (t) => t.serverId);
    return new chatv1.DeleteServerResponse();
  }

  getServer(req: chatv1.GetServerRequest): chatv1.GetServerResponse {
    return new chatv1.GetServerResponse({ server: this.findServer(req.id) });
  }

  listServers(): chatv1.ListServersResponse {
    return new chatv1.ListServersResponse({ servers: this.servers });
  }

  async getServerIcon(req: chatv1.GetServerIconRequest): Promise<chatv1.GetServerIconResponse> {
    this.findServer(req.serverId);
    if (!this.icons.has(req.serverId)) {
      this.icons.set(req.serverId, await serverIcon(req.serverId));
    }
    return new chatv1.GetServerIconResponse({
      serverIcon: this.icons.get(req.serverId) ?? undefined,
    });
  }

  updateServerIcon(req: chatv1.UpdateServerIconRequest): chatv1.UpdateServerIconResponse {
    this.findServer(req.serverId);
    const icon = new ServerIcon({ id: req.serverId, image: req.image });
    this.icons.set(req.serverId, icon);
    return new chatv1.UpdateServerIconResponse({ serverIcon: icon });
  }

  createEmote(req: chatv1.CreateEmoteRequest): chatv1.CreateEmoteResponse {
    this.findServer(req.serverId);
    const e = new Emote({ ...req, id: this.newId() });
    this.emotes.set(e.id, { emote: e });
    return new chatv1.CreateEmoteResponse({ emote: e });
  }

  updateEmote(req: chatv1.UpdateEmoteRequest): chatv1.UpdateEmoteResponse {
    this.find(this.emotes, "emote", req.id);
    const e = new Emote({ ...req });
    this.emotes.set(e.id, { emote: e });
    return new chatv1.UpdateEmoteResponse({ emote: e });
  }

  deleteEmote(req: chatv1.DeleteEmoteRequest): chatv1.DeleteEmoteResponse {
    this.find(this.emotes, "emote", req.id);
    this.emotes.delete(req.id);
    return new chatv1.DeleteEmoteResponse();
  }

  async getEmote(req: chatv1.GetEmoteRequest): Promise<chatv1.GetEmoteResponse> {
    const e = await this.loadEmote(this.find(this.emotes, "emote", req.id));
    return new chatv1.GetEmoteResponse({ emote: e });
  }

  async listEmotes(req: chatv1.ListEmotesRequest): Promise<chatv1.ListEmotesResponse> {
    const records = this.ofServer(this.emotes, req.serverId, (r) => r.emote.serverId);
    const assets = req.parts.includes(chatv1.ListEmotesRequest.Part.PART_ASSETS);
    const emotes = await Promise.all(
      records.map(async (r) => {
        const e = assets ? await this.loadEmote(r) : r.emote;
        return assets ? e : new Emote({ ...e, images: [] });
      })
    );
    return new chatv1.ListEmotesResponse({ emotes });
  }

  updateEmotes(req: chatv1.UpdateEmotesRequest): chatv1.UpdateEmotesResponse {
    for (const id of req.ids) {
      const r = this.find(this.emotes, "emote", id);
      const labels = new Set(r.emote.labels);
      req.addLabels.forEach((l) => labels.add(l));
      req.removeLabels.forEach((l) => labels.delete(l));
      r.emote = new Emote({
        ...r.emote,
        labels: [...labels],
        enable: req.enable ? req.enable.value : r.emote.enable,
      });
    }
    return new chatv1.UpdateEmotesResponse();
  }

  listEmoteLabels(req: chatv1.ListEmoteLabelsRequest): chatv1.ListEmoteLabelsResponse {
    const labels = new Set<string>();
    for (const { emote } of this.ofServer(this.emotes, req.serverId, (r) => r.emote.serverId)) {
      emote.labels.forEach((l) => labels.add(l));
    }
    return new chatv1.ListEmoteLabelsResponse({ labels: [...labels].sort() });
  }

  createModifier(req: chatv1.CreateModifierRequest): chatv1.CreateModifierResponse {
    this.findServer(req.serverId);
    const modifier = new Modifier({ ...req, id: this.newId() });
    this.modifiers.set(modifier.id, { modifier, assets: [] });
    return new chatv1.CreateModifierResponse({ modifier });
  }

  updateModifier(req: chatv1.UpdateModifierRequest): chatv1.UpdateModifierResponse {
    this.find(this.modifiers, "modifier", req.id);
    const modifier = new Modifier({ ...req });
    this.modifiers.set(modifier.id, { modifier, assets: [] });
    return new chatv1.UpdateModifierResponse({ modifier });
  }

  deleteModifier(req: chatv1.DeleteModifierRequest): chatv1.DeleteModifierResponse {
    this.find(this.modifiers, "modifier", req.id);
    this.modifiers.delete(req.id);
    return new chatv1.DeleteModifierResponse();
  }

  async getModifier(req: chatv1.GetModifierRequest): Promise<chatv1.GetModifierResponse> {
    const r = this.find(this.modifiers, "modifier", req.id);
    return new chatv1.GetModifierResponse({ modifier: await loadAssets(r) });
  }

  async listModifiers(req: chatv1.ListModifiersRequest): Promise<chatv1.ListModifiersResponse> {
    const records = this.ofServer(this.modifiers, req.serverId, (r) => r.modifier.serverId);
    return new chatv1.ListModifiersResponse({
      modifiers: await Promise.all(records.map(loadAssets)),
    });
  }

  createTag(req: chatv1.CreateTagRequest): chatv1.CreateTagResponse {
    this.findServer(req.serverId);
    const tag = new Tag({ ...req, id: this.newId() });
    this.tags.set(tag.id, tag);
    return new chatv1.CreateTagResponse({ tag });
  }

  updateTag(req: chatv1.UpdateTagRequest): chatv1.UpdateTagResponse {
    this.find(this.tags, "tag", req.id);
    const tag = new Tag({ ...req });
    this.tags.set(tag.id, tag);
    return new chatv1.UpdateTagResponse({ tag });
  }

  deleteTag(req: chatv1.DeleteTagRequest): chatv1.DeleteTagResponse {
    this.find(this.tags, "tag", req.id);
    this.tags.delete(req.id);
    return new chatv1.DeleteTagResponse();
  }

  getTag(req: chatv1.GetTagRequest): chatv1.GetTagResponse {
    return new chatv1.GetTagResponse({ tag: this.find(this.tags, "tag", req.id) });
  }

  listTags(req: chatv1.ListTagsRequest): chatv1.ListTagsResponse {
    return new chatv1.ListTagsResponse({
      tags: this.ofServer(this.tags, req.serverId, (t) => t.serverId),
    });
  }
}
