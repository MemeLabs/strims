import React from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { Controller, useForm } from "react-hook-form";
import Select from "react-select";
import CreatableSelect from "react-select/creatable";

import { InputError, InputLabel, TextInput, ToggleInput } from "../../components/Form";

interface SettingsFormData {
  showTime: boolean;
  showFlairIcons: boolean;
  timestampFormat: string;
  maxLines: number;
  notificationWhisper: boolean;
  soundNotificationWhisper: boolean;
  notificationHighlight: boolean;
  soundNotificationHighlight: boolean;
  notificationSoundFile: {
    fileType: string;
    data: string;
  };
  highlight: boolean;
  customHighlight: string[];
  highlightNicks: string[];
  taggedNicks: string[];
  showRemoved: {
    value: string;
    label: string;
  };
  showWhispersInChat: boolean;
  ignoreNicks: string[];
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
  holidayEmoteModifiers: boolean;
  disableSpoilers: boolean;
  viewerStateIndicator: {
    value: string;
    label: string;
  };
  hiddenEmotes: string[];
}

const stateIndicatorOptions = [
  {
    value: "disable",
    label: "Disable",
  },
  {
    value: "bar",
    label: "Bar",
  },
  {
    value: "dot",
    label: "Dot",
  },
  {
    value: "array",
    label: "Array",
  },
];

const showRemovedOptions = [
  {
    value: "remove",
    label: "Remove",
  },
  {
    value: "censor",
    label: "Censor",
  },
  {
    value: "do nothing",
    label: "Do nothing",
  },
];

const SettingsDrawer: React.FC = () => {
  // const [chat] = useChat();

  const { control } = useForm<SettingsFormData>({
    mode: "onBlur",
    defaultValues: {
      showTime: false,
      showFlairIcons: true,
      timestampFormat: "HH:mm",
      maxLines: 250,
      notificationWhisper: true,
      soundNotificationWhisper: false,
      notificationHighlight: true,
      soundNotificationHighlight: false,
      highlight: true,
      customHighlight: [],
      highlightNicks: [],
      taggedNicks: [],
      showRemoved: showRemovedOptions[0],
      showWhispersInChat: true,
      ignoreNicks: [],
      focusMentioned: false,
      notificationTimeout: true,
      ignoreMentions: false,
      autocompleteHelper: true,
      autocompleteEmotePreview: true,
      taggedVisibility: false,
      hideNsfw: false,
      animateForever: true,
      formatterGreen: true,
      formatterEmote: true,
      formatterCombo: true,
      holidayEmoteModifiers: true,
      disableSpoilers: false,
      viewerStateIndicator: stateIndicatorOptions[1],
      hiddenEmotes: [],
    },
  });

  return (
    <Scrollbars autoHide={true}>
      <form className="chat__settings_form">
        {/* {error && <InputError error={error.message || "Error creating chat server"} />} */}
        <fieldset>
          <legend>Messages</legend>
          <ToggleInput control={control} label="Show flair" name="showFlairIcons" />
          <ToggleInput control={control} label="Show time" name="showTime" />
          <ToggleInput control={control} label="Harsh ignore" name="ignoreMentions" />
          <ToggleInput
            control={control}
            label="Hide messages tagged nsfw or nsfl"
            name="hideNsfw"
          />
          <ToggleInput
            control={control}
            label="Loop animated emotes forever"
            name="animateForever"
          />
          <ToggleInput control={control} label="Disable spoilers" name="disableSpoilers" />
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
          />
          <InputLabel text="Stream viewer indicators">
            <Controller
              name="viewerStateIndicator"
              control={control}
              render={({ field, fieldState: { error } }) => (
                <>
                  <Select
                    {...field}
                    className="input_select"
                    classNamePrefix="react_select"
                    options={stateIndicatorOptions}
                  />
                  <InputError error={error} />
                </>
              )}
            />
          </InputLabel>
          <InputLabel text="Banned messages">
            <Controller
              name="showRemoved"
              control={control}
              render={({ field, fieldState: { error } }) => (
                <>
                  <Select
                    {...field}
                    className="input_select"
                    classNamePrefix="react_select"
                    options={showRemovedOptions}
                  />
                  <InputError error={error} />
                </>
              )}
            />
          </InputLabel>
        </fieldset>
        <fieldset>
          <legend>Autocomplete</legend>
          <ToggleInput control={control} label="Auto-complete helper" name="autocompleteHelper" />
          <ToggleInput
            control={control}
            label="Show emote preview"
            name="autocompleteEmotePreview"
          />
        </fieldset>
        <fieldset>
          <legend>Whispers</legend>
          <ToggleInput control={control} label="In-line messages" name="showWhispersInChat" />
        </fieldset>

        <fieldset>
          <legend>Highlights, focus &amp; tags</legend>
          <ToggleInput control={control} label="Highlight when mentioned" name="highlight" />
          <ToggleInput
            control={control}
            label="Include mentions when focused"
            name="ignoreMentions"
          />
          <ToggleInput
            control={control}
            label="Increase visibility of tagged users"
            name="taggedVisibility"
          />
          <InputLabel text="Custom highlights">
            <Controller
              name="customHighlight"
              control={control}
              render={({ field, fieldState: { error } }) => (
                <>
                  <CreatableSelect
                    {...field}
                    isMulti={true}
                    placeholder="Custom highlights"
                    className="input_select"
                    classNamePrefix="react_select"
                  />
                  <InputError error={error} />
                </>
              )}
            />
          </InputLabel>
        </fieldset>
        <fieldset>
          <legend>Autocomplete</legend>
          <ToggleInput control={control} label="Autocomplete helper" name="autocompleteHelper" />
          <ToggleInput
            control={control}
            label="Show emote previews"
            name="autocompleteEmotePreview"
          />
        </fieldset>
        <fieldset>
          <legend>Message formatters</legend>
          <ToggleInput control={control} label="Greentext" name="formatterGreen" />
          <ToggleInput control={control} label="Emotes" name="formatterEmote" />
          <ToggleInput control={control} label="Combos" name="formatterCombo" />
          <ToggleInput control={control} label="Modifiers" name="holidayEmoteModifiers" />
        </fieldset>
      </form>
    </Scrollbars>
  );
};

export default SettingsDrawer;
