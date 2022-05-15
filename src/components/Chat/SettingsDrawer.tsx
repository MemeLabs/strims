// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./SettingsDrawer.scss";

import { Base64 } from "js-base64";
import { pick } from "lodash";
import React, { useEffect } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { useForm } from "react-hook-form";
import { Trans, useTranslation } from "react-i18next";

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
  shortenLinks: boolean;
  compactEmoteSpacing: boolean;
  normalizeAliasCase: boolean;
  viewerStateIndicator: {
    value: UIConfig.ViewerStateIndicator;
    label: string;
  };
}

const primitivePropNames = [
  "showTime",
  "showFlairIcons",
  "maxLines",
  "notificationWhisper",
  "soundNotificationWhisper",
  "notificationHighlight",
  "soundNotificationHighlight",
  "highlight",
  "customHighlight",
  "showWhispersInChat",
  "focusMentioned",
  "notificationTimeout",
  "ignoreMentions",
  "autocompleteHelper",
  "autocompleteEmotePreview",
  "taggedVisibility",
  "hideNsfw",
  "animateForever",
  "formatterGreen",
  "formatterEmote",
  "formatterCombo",
  "emoteModifiers",
  "disableSpoilers",
  "shortenLinks",
  "compactEmoteSpacing",
  "normalizeAliasCase",
] as const;

const SettingsDrawer: React.FC = () => {
  const { t } = useTranslation();

  const viewerStateIndicatorOptions = [
    {
      value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_DISABLED,
      label: t("chat.settings.Disable"),
    },
    {
      value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_BAR,
      label: t("chat.settings.Bar"),
    },
    {
      value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_DOT,
      label: t("chat.settings.Dot"),
    },
    {
      value: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_ARRAY,
      label: t("chat.settings.Array"),
    },
  ];

  const showRemovedOptions = [
    {
      value: UIConfig.ShowRemoved.SHOW_REMOVED_REMOVE,
      label: t("chat.settings.Remove"),
    },
    {
      value: UIConfig.ShowRemoved.SHOW_REMOVED_CENSOR,
      label: t("chat.settings.Censor"),
    },
    {
      value: UIConfig.ShowRemoved.SHOW_REMOVED_DO_NOTHING,
      label: t("chat.settings.Do nothing"),
    },
  ];

  const [{ uiConfig }, { mergeUIConfig }] = useChat();

  const { control, getValues, reset } = useForm<SettingsFormData>({ mode: "onBlur" });

  useEffect(() => {
    const { showRemoved, viewerStateIndicator, notificationSoundFile, ...values } = uiConfig;

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
  }, [uiConfig]);

  const handleChange = () => {
    const {
      showRemoved: { value: showRemoved },
      viewerStateIndicator: { value: viewerStateIndicator },
      notificationSoundFile,
    } = getValues();
    const values = pick(getValues(), primitivePropNames);

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
          <legend>
            <Trans>chat.settings.Messages</Trans>
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.Show flair")}
            name="showFlairIcons"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Show time")}
            name="showTime"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Harsh ignore")}
            name="ignoreMentions"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Hide messages tagged nsfw or nsfl")}
            name="hideNsfw"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Loop animated emotes forever")}
            name="animateForever"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Disable spoilers")}
            name="disableSpoilers"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Shorten long links")}
            name="shortenLinks"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Compact emote spacing")}
            name="compactEmoteSpacing"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Normalize alias capitalization")}
            name="normalizeAliasCase"
            onChange={handleChange}
          />
          <TextInput
            control={control}
            label={t("chat.settings.Maximum messages")}
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
            label={t("chat.settings.Stream viewer indicators")}
            name="viewerStateIndicator"
            options={viewerStateIndicatorOptions}
            onChange={handleChange}
            isSearchable={false}
          />
          <SelectInput
            control={control}
            label={t("chat.settings.Banned messages")}
            name="showRemoved"
            options={showRemovedOptions}
            onChange={handleChange}
            isSearchable={false}
          />
        </fieldset>
        <fieldset>
          <legend>
            <Trans>chat.settings.Whispers</Trans>
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.In-line messages")}
            name="showWhispersInChat"
            onChange={handleChange}
          />
        </fieldset>

        <fieldset>
          <legend>
            <Trans>chat.settings.Highlights, focus, and tags</Trans>
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.Highlight when mentioned")}
            name="highlight"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Include mentions when focused")}
            name="ignoreMentions"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Increase visibility of tagged users")}
            name="taggedVisibility"
            onChange={handleChange}
          />
          <TextAreaInput
            control={control}
            label={t("chat.settings.Custom highlights")}
            name="customHighlight"
            placeholder={t("chat.settings.Comma separated")}
            onBlur={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>
            <Trans>chat.settings.Autocomplete</Trans>
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.Autocomplete helper")}
            name="autocompleteHelper"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Show emote previews")}
            name="autocompleteEmotePreview"
            onChange={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>
            <Trans>chat.settings.Message formatters</Trans>
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.Greentext")}
            name="formatterGreen"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Emotes")}
            name="formatterEmote"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Combos")}
            name="formatterCombo"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.Modifiers")}
            name="emoteModifiers"
            onChange={handleChange}
          />
        </fieldset>
      </form>
    </Scrollbars>
  );
};

export default SettingsDrawer;
