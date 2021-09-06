import { Base64 } from "js-base64";
import React, { useEffect } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { useForm } from "react-hook-form";

import { UIConfig } from "../../apis/strims/chat/v1/chat";
import { SelectInput, TextAreaInput, TextInput, ToggleInput } from "../../components/Form";
import { useChat } from "../../contexts/Chat";

interface SettingsFormData {
  showTime: boolean;
  showFlairIcons: boolean;
  maxLines: number;
  notificationWhisper: boolean;
  soundNotificationWhisper: boolean;
  notificationHighlight: boolean;
  soundNotificationHighlight: boolean;
  notificationSoundFile?: {
    fileType: string;
    data: string;
  };
  highlight: boolean;
  customHighlight: string;
  showRemoved: {
    value: UIConfig.ShowRemoved;
    label: string;
  };
  showWhispersInChat: boolean;
  focusMentioned: boolean;
  notificationTimeout: boolean;
  ignoreMentions: boolean;
  autocompleteHelper: boolean;
  autocompleteEmotePreview: boolean;
  taggedVisibility: boolean;
  hideNsfw: boolean;
  animateForever: boolean;
  formatterGreen: boolean;
  formatterEmote: boolean;
  formatterCombo: boolean;
  emoteModifiers: boolean;
  disableSpoilers: boolean;
  viewerStateIndicator: {
    value: UIConfig.ViewerStateIndicator;
    label: string;
  };
}

const viewerStateIndicatorOptions = [
  {
    value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_DISABLED,
    label: "Disable",
  },
  {
    value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_BAR,
    label: "Bar",
  },
  {
    value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_DOT,
    label: "Dot",
  },
  {
    value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_ARRAY,
    label: "Array",
  },
];

const showRemovedOptions = [
  {
    value: UIConfig.ShowRemoved.SHOW_REMOVED_REMOVE,
    label: "Remove",
  },
  {
    value: UIConfig.ShowRemoved.SHOW_REMOVED_CENSOR,
    label: "Censor",
  },
  {
    value: UIConfig.ShowRemoved.SHOW_REMOVED_DO_NOTHING,
    label: "Do nothing",
  },
];

const SettingsDrawer: React.FC = () => {
  const [chat, { mergeUIConfig }] = useChat();

  const { control, getValues, reset } = useForm<SettingsFormData>({ mode: "onBlur" });

  useEffect(() => {
    const { showRemoved, viewerStateIndicator, notificationSoundFile, ...values } = chat.uiConfig;

    reset({
      showRemoved: showRemovedOptions.find(({ value }) => value === showRemoved),
      viewerStateIndicator: viewerStateIndicatorOptions.find(
        ({ value }) => value === viewerStateIndicator
      ),
      notificationSoundFile: notificationSoundFile?.data
        ? {
            fileType: notificationSoundFile.fileType,
            data: Base64.fromUint8Array(notificationSoundFile.data),
          }
        : undefined,
      ...values,
    });
  }, [chat.uiConfig]);

  const handleChange = () => {
    const {
      showRemoved: { value: showRemoved },
      viewerStateIndicator: { value: viewerStateIndicator },
      notificationSoundFile,
      ...values
    } = getValues();

    mergeUIConfig({
      showRemoved,
      viewerStateIndicator,
      notificationSoundFile: notificationSoundFile
        ? new UIConfig.SoundFile({
            fileType: notificationSoundFile.fileType,
            data: Base64.toUint8Array(notificationSoundFile.data),
          })
        : undefined,
      ...values,
    });
  };

  return (
    <Scrollbars autoHide={true}>
      <form className="chat__settings_form">
        {/* {error && <InputError error={error.message || "Error creating chat server"} />} */}
        <fieldset>
          <legend>Messages</legend>
          <ToggleInput
            control={control}
            label="Show flair"
            name="showFlairIcons"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Show time"
            name="showTime"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Harsh ignore"
            name="ignoreMentions"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Hide messages tagged nsfw or nsfl"
            name="hideNsfw"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Loop animated emotes forever"
            name="animateForever"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Disable spoilers"
            name="disableSpoilers"
            onChange={handleChange}
          />
          <TextInput
            control={control}
            label="Maximum messages"
            name="maxLines"
            type="number"
            rules={{
              pattern: {
                value: /^\d+$/i,
                message: "Maximum messages must be numeric",
              },
            }}
            onBlur={handleChange}
          />
          <SelectInput
            control={control}
            label="Stream viewer indicators"
            name="viewerStateIndicator"
            options={viewerStateIndicatorOptions}
            onChange={handleChange}
          />
          <SelectInput
            control={control}
            label="Banned messages"
            name="showRemoved"
            options={showRemovedOptions}
            onChange={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>Autocomplete</legend>
          <ToggleInput
            control={control}
            label="Auto-complete helper"
            name="autocompleteHelper"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Show emote preview"
            name="autocompleteEmotePreview"
            onChange={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>Whispers</legend>
          <ToggleInput
            control={control}
            label="In-line messages"
            name="showWhispersInChat"
            onChange={handleChange}
          />
        </fieldset>

        <fieldset>
          <legend>Highlights, focus &amp; tags</legend>
          <ToggleInput
            control={control}
            label="Highlight when mentioned"
            name="highlight"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Include mentions when focused"
            name="ignoreMentions"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Increase visibility of tagged users"
            name="taggedVisibility"
            onChange={handleChange}
          />
          <TextAreaInput
            control={control}
            label="Custom highlights"
            name="customHighlight"
            placeholder="Comma separated..."
            onBlur={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>Autocomplete</legend>
          <ToggleInput
            control={control}
            label="Autocomplete helper"
            name="autocompleteHelper"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Show emote previews"
            name="autocompleteEmotePreview"
            onChange={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>Message formatters</legend>
          <ToggleInput
            control={control}
            label="Greentext"
            name="formatterGreen"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Emotes"
            name="formatterEmote"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Combos"
            name="formatterCombo"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label="Modifiers"
            name="emoteModifiers"
            onChange={handleChange}
          />
        </fieldset>
      </form>
    </Scrollbars>
  );
};

export default SettingsDrawer;
