// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { MouseEvent, ReactNode, useCallback, useMemo } from "react";
import Dropzone from "react-dropzone";
import { Control, useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { EmoteScale } from "../../../apis/strims/chat/v1/chat";
import {
  Button,
  ButtonSet,
  CreatableSelectInput,
  ImageInput,
  ImageValue,
  InputError,
  InputLabel,
  SelectOption,
  TextInput,
} from "../../../components/Form";
import { ImageInputPlaceholder } from "../../../components/Form/ImageInput";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { mimeTypeToFileType } from "./utils";

type EmoteIndex = 0 | 1 | 2 | 3 | 4;
type Values<T> = T[keyof T];

type EmoteNameInputs = Partial<Record<Values<{ [I in EmoteIndex]: `emote_${I}_name` }>, string>>;
type EmoteImageInputs = Partial<
  Record<Values<{ [I in EmoteIndex]: `emote_${I}_image` }>, ImageValue>
>;

interface ChatEmoteBulkFormData extends EmoteNameInputs, EmoteImageInputs {
  labels: SelectOption<string>[];
  count: number;
}

const ChatEmoteCreateFormPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const serverId = BigInt(useParams<"serverId">().serverId);
  const [{ value }] = useCall("chatServer", "listEmotes", {
    args: [{ serverId }],
  });
  const navigate = useNavigate();
  const [{ error, loading }, createChatEmote] = useLazyCall("chatServer", "createEmote");

  const onSubmit = async (data: ChatEmoteBulkFormData) => {
    for (let i = 0; i < data.count; i++) {
      const image = data[`emote_${i as EmoteIndex}_image`];
      const name = data[`emote_${i as EmoteIndex}_name`];
      await createChatEmote({
        serverId,
        name,
        images: [
          {
            data: Base64.toUint8Array(image.data),
            fileType: mimeTypeToFileType(image.type),
            height: image.height,
            width: image.width,
            scale:
              image.height > 100
                ? EmoteScale.EMOTE_SCALE_4X
                : image.height > 50
                ? EmoteScale.EMOTE_SCALE_2X
                : EmoteScale.EMOTE_SCALE_1X,
          },
        ],
        labels: data.labels?.map(({ value }) => value),
      });
    }
    navigate(`/settings/chat-servers/${serverId}/emotes`, { replace: true });
  };

  const [listLabelsRes] = useCall("chatServer", "listEmoteLabels", { args: [{ serverId }] });
  const labelOptions: SelectOption<string>[] = useMemo(
    () => listLabelsRes.value?.labels.map((label) => ({ label, value: label })),
    [listLabelsRes.value]
  );

  const backLink = value?.emotes.length
    ? `/settings/chat-servers/${serverId}/emotes`
    : `/settings/chat-servers/${serverId}`;

  const { handleSubmit, control, setValue, getValues, watch } = useForm<ChatEmoteBulkFormData>({
    mode: "onBlur",
    defaultValues: {
      count: 0,
    },
  });

  const createEmote = useCallback((name: string, image: ImageValue) => {
    const index = (getValues().count >> 0) as EmoteIndex;
    setValue(`emote_${index}_name`, name);
    setValue(`emote_${index}_image`, image);
    setValue("count", index + 1);
  }, []);

  const handleEmoteDelete = useCallback((i: EmoteIndex) => {
    const values = getValues();
    for (let j = i; j < values.count; j++) {
      const k = (j + 1) as EmoteIndex;
      setValue(`emote_${j}_name`, values[`emote_${k}_name`]);
      setValue(`emote_${j}_image`, values[`emote_${k}_image`]);
    }
    setValue("count", values.count - 1);
  }, []);

  const count = watch("count");

  const emoteInputs: ReactNode[] = [];
  for (let i = 0; i < count; i++) {
    emoteInputs.push(
      <EmoteInput control={control} key={i} index={i as EmoteIndex} onDelete={handleEmoteDelete} />
    );
  }

  const handleDrop = async (files: File[]) => {
    if (files.length === 0) {
      return;
    }

    for (const file of files) {
      const url = URL.createObjectURL(file);

      const data = await new Promise<ArrayBuffer>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
        reader.onerror = () => reject();
        reader.readAsArrayBuffer(file);
      });

      const img = new Image();
      img.src = url;
      img.onload = () => {
        const name = file.name.replace(/\.\w+$/, "");
        const image = {
          data: Base64.fromUint8Array(new Uint8Array(data)),
          type: file.type,
          height: img.height,
          width: img.width,
        };
        createEmote(name, image);
      };
    }
  };

  return (
    <>
      <TableTitleBar label="Create Emotes" backLink={backLink} />
      <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
        {error && <InputError error={error.message || "Error creating chat server"} />}
        <CreatableSelectInput
          control={control}
          name="labels"
          label="Labels"
          options={labelOptions}
        />
        <InputLabel text="Images" component="div">
          <Dropzone
            maxSize={10485764}
            multiple={true}
            accept={{ "image/*": [] }}
            onDrop={handleDrop}
          >
            {({ getRootProps, getInputProps }) => (
              <div {...getRootProps()} className="input_image">
                <ImageInputPlaceholder classNameBase="input_image" />
                <input name="file" {...getInputProps()} />
              </div>
            )}
          </Dropzone>
        </InputLabel>
        {...emoteInputs}
        <ButtonSet>
          <Button disabled={loading}>Create Emotes</Button>
        </ButtonSet>
      </form>
    </>
  );
};

export default ChatEmoteCreateFormPage;

interface EmoteInputProps {
  control: Control<ChatEmoteBulkFormData>;
  index: EmoteIndex;
  onDelete: (i: number) => void;
}

const EmoteInput: React.FC<EmoteInputProps> = ({ control, index, onDelete }) => {
  const handleDeleteClick = useCallback((e: MouseEvent) => {
    e.preventDefault();
    onDelete(index);
  }, []);

  return (
    <>
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Emote name required",
          },
        }}
        name={`emote_${index}_name`}
        label="Name"
        placeholder="Enter an emote name"
      />
      <InputLabel text="image" component="div">
        <ImageInput
          control={control}
          name={`emote_${index}_image`}
          maxSize={10485764}
          rules={{
            required: {
              value: true,
              message: "Image is required",
            },
          }}
        />
      </InputLabel>
      <ButtonSet>
        <Button onClick={handleDeleteClick}>Remove Emote</Button>
      </ButtonSet>
    </>
  );
};
