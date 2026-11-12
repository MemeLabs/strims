// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Base64 } from "js-base64";
import React, { useEffect, useRef, useState } from "react";

import { ImageValue } from "../../../../../components/Form";
import VideoChannelForm, {
  VideoChannelFormData,
  VideoChannelFormProps,
  themeColorOptions,
} from "../../../../../pages/Settings/Video/VideoChannelForm";
import NetworkService from "../../../../mocks/network/service";
import { MockApi, Story, fixtures } from "../../../../storybook";
import { SettingsContent, longTitle, useSubmitOnLoad } from "../Layout.harness";

// Fixture icon `i` as the logo input holds an upload.
const useImageValue = (i: number): ImageValue | null => {
  const [value, setValue] = useState<ImageValue>(null);
  useEffect(() => {
    void (async () => {
      const data = await fixtures.imageData(fixtures.iconUrls[i]);
      const { width, height } = await createImageBitmap(
        new Blob([data as Uint8Array<ArrayBuffer>])
      );
      setValue({ data: Base64.fromUint8Array(data), type: "image/png", width, height });
    })();
  }, []);
  return value;
};

const filled = (channelLogo: ImageValue): VideoChannelFormData => ({
  channelName: "test_user",
  channelLogo,
  title: "Any% practice, chill vibes",
  description: "Grinding the new route. Splits in the description, questions welcome.",
  tags: ["speedrun", "games"].map((value) => ({ label: value, value })),
  networkKey: Base64.fromUint8Array(fixtures.networkKey(2)),
  themeColor: themeColorOptions[1],
});

// Past the 100 character name and title and the 500 character description.
const tooLong = (channelLogo: ImageValue): VideoChannelFormData => ({
  ...filled(channelLogo),
  channelName: "a_channel_name_".repeat(8),
  title: longTitle,
  description: Array(8)
    .fill("A description longer than the five hundred characters it may have.")
    .join(" "),
  tags: ["speedrun", "games", "any%", "practice", "chill", "no commentary", "pb attempts"].map(
    (value) => ({ label: value, value })
  ),
});

interface ContextProps extends Partial<VideoChannelFormProps> {
  /** The fixture icon the values' logo is; values wait for it to load. */
  logo?: number;
  makeValues?: (logo: ImageValue) => VideoChannelFormData;
  submit?: boolean;
}

// The form in a settings page over five fixture networks; submitting it
// does nothing.
const Context: React.FC<ContextProps> = ({ logo = 0, makeValues, submit, ...props }) => {
  const ref = useRef<HTMLDivElement>(null);
  useSubmitOnLoad(ref, submit);
  const image = useImageValue(logo);
  if (makeValues && !image) {
    return null;
  }
  return (
    <MockApi services={() => ({ network: new NetworkService({ count: 5 }) })}>
      <SettingsContent ref={ref}>
        <VideoChannelForm
          values={makeValues?.(image)}
          onSubmit={() => undefined}
          error={null}
          loading={false}
          submitLabel="Create Channel"
          {...props}
        />
      </SettingsContent>
    </MockApi>
  );
};

const stories: Story[] = [
  {
    name: "New",
    component: () => <Context />,
  },
  {
    name: "Filled",
    component: () => <Context makeValues={filled} submitLabel="Update Channel" />,
  },
  {
    // Submitted empty.
    name: "Invalid",
    component: () => <Context submit />,
  },
  {
    // Submitted with fields past their limits.
    name: "Too long",
    component: () => <Context makeValues={tooLong} submitLabel="Update Channel" submit />,
  },
  {
    // The save's error, as the create and edit pages pass it.
    name: "Error",
    component: () => (
      <Context makeValues={filled} error={new Error({ message: "channel limit reached" })} />
    ),
  },
];

export default stories;
