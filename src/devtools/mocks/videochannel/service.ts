// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import {
  IListingSnippet,
  ListingSnippetImage,
} from "../../../apis/strims/network/v1/directory/directory";
import { ImageType } from "../../../apis/strims/type/image";
import { Key, KeyType } from "../../../apis/strims/type/key";
import * as channelv1 from "../../../apis/strims/video/v1/channel";
import {
  UnimplementedVideoChannelFrontendService,
  VideoChannelFrontendService,
} from "../../../apis/strims/video/v1/channel_rpc";
import { bytes, key } from "../../storybook/fixtures/ids";
import { iconUrls, imageData } from "../../storybook/fixtures/images";
import { networkKey } from "../../storybook/fixtures/networks";

const snippets: IListingSnippet[] = [
  {
    title: "Any% practice, chill vibes",
    description: "Grinding the new route. Splits in the description, questions welcome.",
    tags: ["speedrun", "games"],
    themeColor: 0xff0000,
  },
  {
    title: "Late night lofi",
    description: "Beats to study and code to.",
    tags: ["music", "lofi"],
    themeColor: 0x0000ff,
  },
  {
    title: "Film club: Stalker (1979)",
    description: "",
    tags: ["film"],
    themeColor: 0x00ff00,
  },
  {
    title: "Retro marathon",
    description: "Every console under the desk, one after the other.",
    tags: [],
    themeColor: 0x000000,
  },
];

// Channel `i`, from 1: a local channel of fixture network ((i - 1) % 5) + 1
// whose logo, once `withLogo` gives it one, is fixture icon i - 1.
export const videoChannel = (i: number, snippet: IListingSnippet = {}): channelv1.VideoChannel =>
  new channelv1.VideoChannel({
    id: BigInt(i),
    owner: {
      local: { authKey: bytes(`channel:${i}:auth`, 32), networkKey: networkKey(((i - 1) % 5) + 1) },
    },
    key: new Key({
      type: KeyType.KEY_TYPE_ED25519,
      public: key(`channel:${i}`),
      private: bytes(`channel:${i}:private`, 64),
    }),
    token: bytes(`channel:${i}:token`, 32),
    directoryListingSnippet: {
      channelName: "test_user",
      ...snippets[(i - 1) % snippets.length],
      ...(i > snippets.length
        ? { title: `${snippets[(i - 1) % snippets.length].title} ${i}` }
        : {}),
      ...snippet,
    },
  });

// Channels 1..count.
export const videoChannels = (count: number): channelv1.VideoChannel[] =>
  Array.from({ length: count }, (_, i) => videoChannel(i + 1));

// `channel` with an image logo: `url`'s bytes, as a channel form uploads it.
export const withLogo = async (
  channel: channelv1.VideoChannel,
  url = iconUrls[Number(channel.id - BigInt(1)) % iconUrls.length]
): Promise<channelv1.VideoChannel> => {
  const data = await imageData(url);
  const { width, height } = await createImageBitmap(new Blob([data as Uint8Array<ArrayBuffer>]));
  const c = channelv1.VideoChannel.decode(channelv1.VideoChannel.encode(channel).finish());
  c.directoryListingSnippet.channelLogo = new ListingSnippetImage({
    sourceOneof: { image: { data, type: ImageType.IMAGE_TYPE_PNG, width, height } },
  });
  return c;
};

export interface VideoChannelServiceOptions {
  /** The channels it starts with, as given; default videoChannels(3) with logos. */
  channels?: channelv1.VideoChannel[];
}

const errNotFound = (id: bigint) => new Error(`channel ${id} not found`);

// One node's video channels. The default channels' logos load before any
// call answers.
export default class VideoChannelService
  extends UnimplementedVideoChannelFrontendService
  implements VideoChannelFrontendService
{
  channels: channelv1.VideoChannel[];
  nextId: number;
  ready: Promise<void>;

  constructor({ channels }: VideoChannelServiceOptions = {}) {
    super();
    this.channels = channels ?? videoChannels(3);
    this.nextId = Math.max(0, ...this.channels.map((c) => Number(c.id))) + 1;
    this.ready = channels ? Promise.resolve() : this.loadLogos();
  }

  private async loadLogos(): Promise<void> {
    this.channels = await Promise.all(this.channels.map((c) => withLogo(c)));
  }

  private find(id: bigint): channelv1.VideoChannel {
    const channel = this.channels.find((c) => c.id === id);
    if (!channel) {
      throw errNotFound(id);
    }
    return channel;
  }

  async list(): Promise<channelv1.VideoChannelListResponse> {
    await this.ready;
    return new channelv1.VideoChannelListResponse({ channels: this.channels });
  }

  async get(req: channelv1.VideoChannelGetRequest): Promise<channelv1.VideoChannelGetResponse> {
    await this.ready;
    return new channelv1.VideoChannelGetResponse({ channel: this.find(req.id) });
  }

  create(req: channelv1.VideoChannelCreateRequest): channelv1.VideoChannelCreateResponse {
    const id = this.nextId++;
    const { key, token } = videoChannel(id);
    const channel = new channelv1.VideoChannel({
      id: BigInt(id),
      key,
      token,
      owner: { local: { authKey: bytes(`channel:${id}:auth`, 32), networkKey: req.networkKey } },
      directoryListingSnippet: req.directoryListingSnippet,
    });
    this.channels = [...this.channels, channel];
    return new channelv1.VideoChannelCreateResponse({ channel });
  }

  update(req: channelv1.VideoChannelUpdateRequest): channelv1.VideoChannelUpdateResponse {
    const prev = this.find(req.id);
    const authKey =
      prev.owner.case === channelv1.VideoChannel.OwnerCase.LOCAL
        ? prev.owner.local.authKey
        : bytes(`channel:${req.id}:auth`, 32);
    const channel = new channelv1.VideoChannel({
      id: prev.id,
      key: prev.key,
      token: prev.token,
      owner: { local: { authKey, networkKey: req.networkKey } },
      directoryListingSnippet: req.directoryListingSnippet,
    });
    this.channels = this.channels.map((c) => (c.id === req.id ? channel : c));
    return new channelv1.VideoChannelUpdateResponse({ channel });
  }

  delete(req: channelv1.VideoChannelDeleteRequest): channelv1.VideoChannelDeleteResponse {
    this.find(req.id);
    this.channels = this.channels.filter((c) => c.id !== req.id);
    return new channelv1.VideoChannelDeleteResponse();
  }
}
