import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { useForm } from "react-hook-form";

import { EmoteScale } from "../../../apis/strims/chat/v1/chat";
import {
  CreatableSelectInput,
  ImageInput,
  ImageValue,
  InputError,
  InputLabel,
  SelectInput,
  SelectOption,
  TextAreaInput,
  TextInput,
  ToggleInput,
} from "../../../components/Form";
import BackLink from "./BackLink";

export interface ChatEmoteFormData {
  name: string;
  image: ImageValue;
  scale: {
    value: EmoteScale;
    label: string;
  };
  contributor: string;
  contributorLink: string;
  css: string;
  animated: boolean;
  animationFrameCount: number;
  animationDuration: number;
  animationIterationCount: number;
  animationEndOnFrame: number;
  animationLoopForever: boolean;
  animationAlternateDirection: boolean;
  defaultModifiers: SelectOption<string>[];
}

const scaleOptions = [
  {
    value: EmoteScale.EMOTE_SCALE_1X,
    label: "1x",
  },
  {
    value: EmoteScale.EMOTE_SCALE_2X,
    label: "2x",
  },
  {
    value: EmoteScale.EMOTE_SCALE_4X,
    label: "4x",
  },
];

export interface ChatEmoteFormProps {
  onSubmit: (data: ChatEmoteFormData) => void;
  error: Error;
  loading: boolean;
  serverId: bigint;
  values?: ChatEmoteFormData;
  indexLinkVisible: boolean;
}

const ChatEmoteForm: React.FC<ChatEmoteFormProps> = ({
  onSubmit,
  error,
  loading,
  serverId,
  values = {},
  indexLinkVisible,
}) => {
  const { handleSubmit, control, watch } = useForm<ChatEmoteFormData>({
    mode: "onBlur",
    defaultValues: {
      scale: scaleOptions[0],
      ...values,
    },
  });

  const animated = watch("animated");

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating chat server"} />}
      {indexLinkVisible ? (
        <BackLink
          to={`/settings/chat-servers/${serverId}/emotes`}
          title="Emotes"
          description="Some description of emotes..."
        />
      ) : (
        <BackLink
          to={`/settings/chat-servers/${serverId}`}
          title="Server"
          description="Some description of server..."
        />
      )}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Name is required",
          },
        }}
        name="name"
        label="Name"
        placeholder="Enter a emote name"
      />
      <InputLabel required={true} text="Image" component="div">
        <ImageInput control={control} name="image" maxSize={10485764} />
      </InputLabel>
      <SelectInput control={control} name="scale" label="Scale" options={scaleOptions} />
      <TextInput control={control} label="contributor" name="contributor" />
      <TextInput control={control} label="contributor link" name="contributorLink" />
      <TextAreaInput control={control} label="css" name="css" />
      <ToggleInput control={control} label="animated" name="animated" />
      <TextInput
        control={control}
        label="frame count"
        name="animationFrameCount"
        type="number"
        disabled={!animated}
      />
      <TextInput
        control={control}
        label="duration (ms)"
        name="animationDuration"
        type="number"
        disabled={!animated}
      />
      <TextInput
        control={control}
        label="loops"
        name="animationIterationCount"
        type="number"
        disabled={!animated}
      />
      <TextInput
        control={control}
        label="end on frame"
        name="animationEndOnFrame"
        type="number"
        disabled={!animated}
      />
      <ToggleInput
        control={control}
        label="loop forever"
        name="animationLoopForever"
        disabled={!animated}
      />
      <ToggleInput
        control={control}
        label="alternate directions"
        name="animationAlternateDirection"
        disabled={!animated}
      />
      <CreatableSelectInput
        control={control}
        name="defaultModifiers"
        label="Default modifiers"
        placeholder="Modifiers"
      />
      <label className="input_label">
        <div className="input_label__body">
          <button className="input input_button" disabled={loading}>
            {values ? "Update Emote" : "Create Emote"}
          </button>
        </div>
      </label>
    </form>
  );
};

export default ChatEmoteForm;
