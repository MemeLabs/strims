// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./SettingsDrawer.scss";

import { Base64 } from "js-base64";
import { pick } from "lodash";
import React, { useEffect, useRef } from "react";
import { useForm } from "react-hook-form";
import { Trans, useTranslation } from "react-i18next";

import { UIConfig } from "../../apis/strims/chat/v1/chat";
import { SelectInput, TextAreaInput, TextInput, ToggleInput } from "../../components/Form";
import { useChat } from "../../contexts/Chat";
import Scrollbars from "../Scrollbars";

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
  userPresenceIndicator: {
    value: UIConfig.UserPresenceIndicator;
    label: string;
  };
  replicate: boolean;
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
  "replicate",
] as const;

const SettingsDrawer: React.FC = () => {
  const { t } = useTranslation();

  const userPresenceIndicatorOptions = [
    {
      value: UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_DISABLED,
      label: t("chat.settings.indicatorDisabled"),
    },
    {
      value: UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_BAR,
      label: t("chat.settings.indicatorBar"),
    },
    {
      value: UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_DOT,
      label: t("chat.settings.indicatorDot"),
    },
    {
      value: UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_ARRAY,
      label: t("chat.settings.indicatorArray"),
    },
  ];

  const showRemovedOptions = [
    {
      value: UIConfig.ShowRemoved.SHOW_REMOVED_REMOVE,
      label: t("chat.settings.bannedRemove"),
    },
    {
      value: UIConfig.ShowRemoved.SHOW_REMOVED_CENSOR,
      label: t("chat.settings.bannedCensor"),
    },
    {
      value: UIConfig.ShowRemoved.SHOW_REMOVED_DO_NOTHING,
      label: t("chat.settings.bannedDoNothing"),
    },
  ];

  const [{ uiConfig }, { mergeUIConfig }] = useChat();

  const { control, getValues, reset } = useForm<SettingsFormData>({ mode: "onBlur" });

  useEffect(() => {
    const {
      showRemoved,
      userPresenceIndicator,
      notificationSoundFile,
      customHighlight,
      ...values
    } = uiConfig;

    reset({
      showRemoved: showRemovedOptions.find(({ value }) => value === showRemoved),
      userPresenceIndicator: userPresenceIndicatorOptions.find(
        ({ value }) => value === userPresenceIndicator
      ),
      notificationSoundFile: notificationSoundFile?.data
        ? {
            fileType: notificationSoundFile.fileType,
            data: Base64.fromUint8Array(notificationSoundFile.data),
          }
        : undefined,
      customHighlight: customHighlight.join(", "),
      ...values,
    });
  }, [uiConfig]);

  const handleChange = () => {
    const {
      showRemoved: { value: showRemoved },
      userPresenceIndicator: { value: userPresenceIndicator },
      notificationSoundFile,
      customHighlight,
    } = getValues();
    const values = pick(getValues(), primitivePropNames);

    mergeUIConfig({
      showRemoved,
      userPresenceIndicator,
      notificationSoundFile: notificationSoundFile
        ? new UIConfig.SoundFile({
            fileType: notificationSoundFile.fileType,
            data: Base64.toUint8Array(notificationSoundFile.data),
          })
        : undefined,
      customHighlight: customHighlight.split(/\s*,\s*/).filter(Boolean),
      ...values,
    });
  };

  useEffect(() => handleChange, []);

  const ref = useRef<HTMLFormElement>(null);

  return (
    <Scrollbars autoHide={true}>
      <form className="chat__settings_form" ref={ref}>
        {/* {error && <InputError error={error.message || "Error creating chat server"} />} */}
        <fieldset>
          <legend>
            <Trans i18nKey="chat.settings.messagesSection" />
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.showFlair")}
            name="showFlairIcons"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.showTime")}
            name="showTime"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.harshIgnore")}
            name="ignoreMentions"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.hideNsfw")}
            name="hideNsfw"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.loopAnimatedEmotes")}
            name="animateForever"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.disableSpoilers")}
            name="disableSpoilers"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.shortenLinks")}
            name="shortenLinks"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.compactEmoteSpacing")}
            name="compactEmoteSpacing"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.normalizeAliasCase")}
            name="normalizeAliasCase"
            onChange={handleChange}
          />
          <TextInput
            control={control}
            label={t("chat.settings.maxMessages")}
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
            label={t("chat.settings.viewerIndicators")}
            name="userPresenceIndicator"
            options={userPresenceIndicatorOptions}
            onChange={handleChange}
            isSearchable={false}
            menuPortalTarget={ref.current}
          />
          <SelectInput
            control={control}
            label={t("chat.settings.bannedMessages")}
            name="showRemoved"
            options={showRemovedOptions}
            onChange={handleChange}
            isSearchable={false}
            menuPortalTarget={ref.current}
          />
        </fieldset>
        <fieldset>
          <legend>
            <Trans i18nKey="chat.settings.whispersSection" />
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.inlineWhispers")}
            name="showWhispersInChat"
            onChange={handleChange}
          />
        </fieldset>

        <fieldset>
          <legend>
            <Trans i18nKey="chat.settings.highlightsSection" />
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.highlightMentions")}
            name="highlight"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.focusMentions")}
            name="focusMentioned"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.taggedVisibility")}
            name="taggedVisibility"
            onChange={handleChange}
          />
          <TextAreaInput
            control={control}
            label={t("chat.settings.customHighlights")}
            name="customHighlight"
            placeholder={t("chat.settings.customHighlightsPlaceholder")}
            onBlur={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>
            <Trans i18nKey="chat.settings.autocompleteSection" />
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.autocompleteHelper")}
            name="autocompleteHelper"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.emotePreviews")}
            name="autocompleteEmotePreview"
            onChange={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>
            <Trans i18nKey="chat.settings.formattersSection" />
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.greentext")}
            name="formatterGreen"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.emotes")}
            name="formatterEmote"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.combos")}
            name="formatterCombo"
            onChange={handleChange}
          />
          <ToggleInput
            control={control}
            label={t("chat.settings.modifiers")}
            name="emoteModifiers"
            onChange={handleChange}
          />
        </fieldset>
        <fieldset>
          <legend>
            <Trans i18nKey="chat.settings.settingsSection" />
          </legend>
          <ToggleInput
            control={control}
            label={t("chat.settings.keepSynced")}
            name="replicate"
            onChange={handleChange}
          />
        </fieldset>
      </form>
    </Scrollbars>
  );
};

export default SettingsDrawer;
