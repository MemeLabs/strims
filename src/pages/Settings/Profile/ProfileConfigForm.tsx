// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import ForwardLink from "../ForwardLink";

const ProfileConfigForm = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  return (
    <>
      <TableTitleBar label="Profile" />
      <div className="thing_form">
        <ForwardLink to="/settings/profile/pairing-token" title="Generate pairing token" />
        <ForwardLink to="/settings/profile/devices" title="Devices" />
      </div>
    </>
  );
};

export default ProfileConfigForm;
