// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { ToggleInput } from "../../components/Form";
import { TableTitleBar } from "../../components/Settings/Table";
import { useLayout } from "../../contexts/Layout";

interface AppearanceFormData {
  swapMainPanels: boolean;
}

// The device's own layout choices, kept by the layout context in the page's
// local storage. A toggle applies as it is flipped, and follows the layout
// when a hotkey changes it.
const AppearancePage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.appearance.title"));

  const { swapMainPanels, toggleSwapMainPanels } = useLayout();
  const { control, reset } = useForm<AppearanceFormData>({
    defaultValues: { swapMainPanels },
  });
  useEffect(() => reset({ swapMainPanels }), [swapMainPanels]);

  return (
    <>
      <TableTitleBar label="Appearance" />
      <form className="thing_form" onSubmit={(e) => e.preventDefault()}>
        <ToggleInput
          control={control}
          label={t("settings.appearance.chatOnLeft")}
          description={t("settings.appearance.chatOnLeftDescription")}
          name="swapMainPanels"
          onChange={(e) => toggleSwapMainPanels(e.target.checked)}
        />
      </form>
    </>
  );
};

export default AppearancePage;
