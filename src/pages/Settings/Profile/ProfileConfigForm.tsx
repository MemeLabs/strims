// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import InternalLink from "../../../components/InternalLink";
import { TableTitleBar } from "../../../components/Settings/Table";

const ProfileConfigForm = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  return (
    <>
      <TableTitleBar label="Profile" />
      <div className="thing_form">
        <InternalLink to="/settings/profile/pairing-token">Generate pairing token</InternalLink>
        <InternalLink to="/settings/profile/devices">Devices</InternalLink>
      </div>
    </>
  );
};

export default ProfileConfigForm;
