// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import {
  Emote,
  EmoteContributor,
  EmoteEffect,
  EmoteFileType,
  EmoteImage,
  EmoteScale,
  IEmoteEffect,
  Modifier,
  Room,
  Server,
  ServerIcon,
  StyleSheet,
  Tag,
} from "../../../apis/strims/chat/v1/chat";
import { Image, ImageType } from "../../../apis/strims/type/image";
import { Key, KeyType } from "../../../apis/strims/type/key";
import {
  iconUrls,
  imageData,
  key,
  networkKey,
  networkName,
  profileKey,
} from "../../storybook/fixtures";

// The chat mock's emote images by name (the file's), animated sprite sheets
// over static images of the same name.
const imageUrls = (() => {
  const urls = new Map<string, string>();
  const add = (ctx: __WebpackModuleApi.RequireContext) => {
    for (const k of ctx.keys()) {
      urls.set(k.replace(/^.*\/|\.png$/g, ""), ctx(k) as string);
    }
  };
  add(require.context("../chat/emotes/static", false, /\.png$/));
  add(require.context("../chat/emotes/animated", false, /\.png$/));
  return urls;
})();

type Sprite = [frameCount: number, durationMs: number, iterationCount: number];

// The animated images' sprite sheets, as the chat mock's asset bundle plays them.
const sprites: Record<string, Sprite> = {
  BONK: [2, 500, 10],
  catJAM: [158, 6500, 2],
  Cinnabunny: [5, 500, 7],
  ComfyMoobers: [34, 2550, 2],
  DuckJAM: [55, 5500, 1],
  NODDERS: [4, 320, 16],
  NOPERS: [8, 640, 16],
  PeepoRun: [6, 420, 12],
  REE: [39, 3900, 2],
  RIDIN: [8, 2000, 7],
  TANTIES: [10, 1400, 5],
  VroomVroom: [101, 4000, 2],
  WAG: [2, 500, 10],
  WAYTOODANK: [90, 1800, 2],
};

const twitch = new Set([
  "4Head",
  "BabyRage",
  "BibleThump",
  "cmonBruh",
  "DatGeoff",
  "haHAA",
  "Kappa",
  "Klappa",
  "Kreygasm",
  "LUL",
  "NotLikeThis",
  "OMEGALUL",
  "PogChamp",
  "SMOrc",
]);

const anime = new Set([
  "AYAYA",
  "AYAWeird",
  "ComfyAYA",
  "MiyanoBird",
  "MiyanoComfy",
  "NeneLaugh",
  "sataniaLUL",
  "SEMPAI",
  "SUGOI",
  "SUGOwO",
  "umaruCry",
]);

const animals = new Set([
  "ComfyCat",
  "ComfyDog",
  "ComfyFerret",
  "DOGGO",
  "DuckerZ",
  "DuckJAM",
  "FerretLOL",
  "SoDoge",
  "Woof",
  "catJAM",
  "Cinnabunny",
  "WAG",
]);

// An emote's labels, from its name.
const labelsOf = (name: string): string[] => {
  const labels: string[] = [];
  if (sprites[name]) {
    labels.push("animated");
  }
  if (/pep|feels/i.test(name)) {
    labels.push("pepe");
  }
  if (/comfy/i.test(name)) {
    labels.push("comfy");
  }
  if (anime.has(name)) {
    labels.push("anime");
  }
  if (animals.has(name)) {
    labels.push("animals");
  }
  if (twitch.has(name)) {
    labels.push("twitch");
  }
  return labels;
};

const contributors = [
  { name: "jbpratt", link: "https://example.com/jbpratt" },
  { name: "soadsoap", link: "" },
  { name: "widdlez", link: "https://example.com/widdlez" },
];

/** An emote's metadata and where its image is. */
export interface EmoteFixture {
  name: string;
  url: string;
  labels: string[];
  enable: boolean;
  contributor?: { name: string; link: string };
  effects: IEmoteEffect[];
}

const effectsOf = (name: string): IEmoteEffect[] => {
  const effects: IEmoteEffect[] = [];
  const sprite = sprites[name];
  if (sprite) {
    const [frameCount, durationMs, iterationCount] = sprite;
    effects.push({
      effect: {
        spriteAnimation: {
          frameCount,
          durationMs,
          iterationCount,
          loopForever: true,
          alternateDirection: name === "REE",
        },
      },
    });
  }
  if (name === "REE") {
    const scss =
      ".this {\n  animation: ree-anim 200ms 5;\n}\n\n" +
      "@keyframes ree-anim {\n  0% {\n    transform: translateX(0);\n  }\n" +
      "  100% {\n    transform: translateX(3px);\n  }\n}\n";
    effects.push({ effect: { customCss: { styleSheet: { scss, css: scss } } } });
  }
  if (name === "REE" || name === "Kappa") {
    effects.push({ effect: { defaultModifiers: { modifiers: ["rustle"] } } });
  }
  return effects;
};

/** Every fixture emote, by name: all the chat mock's images. */
export const emoteFixtures: EmoteFixture[] = [...imageUrls.keys()]
  .sort((a, b) => a.localeCompare(b, "en", { sensitivity: "base" }))
  .map((name, i) => ({
    name,
    url: imageUrls.get(name),
    labels: labelsOf(name),
    enable: i % 9 !== 4,
    contributor: i % 5 === 2 ? contributors[i % contributors.length] : undefined,
    effects: effectsOf(name),
  }));

// The fixture emote named `name`.
export const emoteFixture = (name: string): EmoteFixture => {
  const emote = emoteFixtures.find((e) => e.name === name);
  if (!emote) {
    throw new Error(`no fixture emote ${name}`);
  }
  return emote;
};

// The id ChatServerService gives fixture emote `name` in fixture server `serverId`.
export const emoteId = (serverId: number, name: string): bigint =>
  BigInt(serverId * 10000 + emoteFixtures.indexOf(emoteFixture(name)) + 1);

// The ids ChatServerService gives fixture server `serverId`'s first modifier and tag.
export const firstModifierId = (serverId: number): bigint => BigInt(serverId * 10000 + 5001);
export const firstTagId = (serverId: number): bigint => BigInt(serverId * 10000 + 8001);

// A PNG's size, from its IHDR chunk.
export const pngSize = (data: Uint8Array): { width: number; height: number } => {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  return { width: view.getUint32(16), height: view.getUint32(20) };
};

// An emote image of the PNG at `url`, at 2x as the fixture images are.
export const emoteImage = async (url: string): Promise<EmoteImage> => {
  const data = await imageData(url);
  return new EmoteImage({
    data,
    fileType: EmoteFileType.FILE_TYPE_PNG,
    scale: EmoteScale.EMOTE_SCALE_2X,
    ...pngSize(data),
  });
};

// An image message of the PNG at `url`.
export const pngImage = async (url: string): Promise<Image> => {
  const data = await imageData(url);
  return new Image({ type: ImageType.IMAGE_TYPE_PNG, data, ...pngSize(data) });
};

// `fixture` as server `serverId`'s emote `id`, without images.
export const emote = (serverId: bigint, id: bigint, fixture: EmoteFixture): Emote =>
  new Emote({
    id,
    serverId,
    name: fixture.name,
    labels: fixture.labels,
    enable: fixture.enable,
    contributor: fixture.contributor && new EmoteContributor(fixture.contributor),
    effects: fixture.effects.map((e) => new EmoteEffect(e)),
  });

const roomNames = ["Test Network Chat", "Strims", "Comfy Corner Lounge"];

// Chat server `i`, from 1, in network `i` (NetworkService's fixture networks).
export const server = (i: number, name?: string): Server =>
  new Server({
    id: BigInt(i),
    networkKey: networkKey(i),
    key: new Key({ type: KeyType.KEY_TYPE_ED25519, public: key(`chat-server:${i}`) }),
    room: new Room({ name: name ?? roomNames[i - 1] ?? `${networkName(i)} Chat` }),
    adminPeerKeys: [profileKey],
  });

// Server `serverId`'s icon: one of the fixture icons.
export const serverIcon = async (serverId: bigint): Promise<ServerIcon> =>
  new ServerIcon({
    id: serverId,
    image: await pngImage(iconUrls[(Number(serverId) * 7) % iconUrls.length]),
  });

interface ModifierFixture {
  name: string;
  priority?: number;
  extraWrapCount?: number;
  internal?: boolean;
  procChance?: number;
  scss?: string;
  /** Names of emote images the style sheet uses as assets. */
  assets?: string[];
}

const modifierFixtures: ModifierFixture[] = [
  { name: "mirror", scss: ".mirror {\n  transform: scaleX(-1);\n}\n" },
  { name: "flip", scss: ".flip {\n  transform: scaleY(-1);\n}\n" },
  { name: "smol" },
  { name: "wide" },
  { name: "rain", priority: 1 },
  { name: "snow", priority: 1 },
  { name: "rustle" },
  { name: "worth", extraWrapCount: 1, priority: 1 },
  { name: "dank", priority: 1 },
  { name: "hyper" },
  { name: "love", extraWrapCount: 1, priority: 1 },
  {
    name: "spin",
    scss:
      ".spin {\n  animation: spin linear 800ms 3;\n}\n\n" +
      "@keyframes spin {\n  0% {\n    transform: rotate(0deg);\n  }\n" +
      "  100% {\n    transform: rotate(360deg);\n  }\n}\n",
  },
  {
    name: "banned",
    priority: 1,
    scss: '.banned:after {\n  content: "";\n  background-image: url("RedCard");\n}\n',
    assets: ["RedCard"],
  },
  { name: "lag" },
  { name: "pause" },
  { name: "slow" },
  { name: "fast" },
  { name: "reverse" },
  { name: "jam", extraWrapCount: 1, priority: 1 },
  { name: "pride" },
  { name: "angel" },
  { name: "gold", priority: 1000 },
  { name: "slide", extraWrapCount: 1 },
  { name: "peek", extraWrapCount: 1 },
  { name: "virus", internal: true, procChance: 0.0001 },
];

export interface ModifierRecord {
  modifier: Modifier;
  /** Asset images to load, by asset name. */
  assets: string[];
}

// Server `serverId`'s first `count` modifiers, ids from `firstId`.
export const modifiers = (serverId: bigint, count: number, firstId: bigint): ModifierRecord[] =>
  modifierFixtures.slice(0, count).map(({ scss, assets = [], ...m }, i) => ({
    modifier: new Modifier({
      ...m,
      id: firstId + BigInt(i),
      serverId,
      styleSheet: new StyleSheet({ scss: scss ?? "", css: scss ?? "" }),
    }),
    assets,
  }));

// A modifier's style sheet with its assets' images loaded.
export const loadAssets = async ({ modifier, assets }: ModifierRecord): Promise<Modifier> =>
  new Modifier({
    ...modifier,
    styleSheet: new StyleSheet({
      ...modifier.styleSheet,
      assets: await Promise.all(
        assets.map(async (name) => ({
          name,
          image: await pngImage(emoteFixture(name).url),
        }))
      ),
    }),
  });

const tagFixtures = [
  { name: "nsfw", color: "#e53935", sensitive: true },
  { name: "nsfl", color: "#8e24aa", sensitive: true },
  { name: "loud", color: "#fdd835", sensitive: false },
  { name: "weeb", color: "#ec407a", sensitive: false },
  { name: "spoiler", color: "#607d8b", sensitive: true },
  { name: "copypasta", color: "#43a047", sensitive: false },
];

// Server `serverId`'s first `count` tags, ids from `firstId`.
export const tags = (serverId: bigint, count: number, firstId: bigint): Tag[] =>
  tagFixtures.slice(0, count).map((t, i) => new Tag({ ...t, id: firstId + BigInt(i), serverId }));
