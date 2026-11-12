// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import QRCode from "qrcode";
import React, { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { PairingToken } from "../../../apis/strims/auth/v1/auth";
import { Notification } from "../../../apis/strims/notification/v1/notification";
import { InputError, InputLabel } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall } from "../../../contexts/FrontendApi";
import { useNotification } from "../../../contexts/Notification";

const ProfilePairingToken = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  const { pushTransientNotification } = useNotification();

  const [createPairingTokenRes] = useCall("replication", "createPairingToken");

  const token = createPairingTokenRes.value?.token;
  const code = useMemo(
    () => token && Base64.fromUint8Array(PairingToken.encode(token).finish()),
    [token]
  );

  // The canvas renders only with a token, so the code is drawn once both are.
  const canvas = useRef<HTMLCanvasElement>();
  useEffect(() => {
    if (code) {
      void QRCode.toCanvas(canvas.current, code, { margin: 15 });
    }
  }, [code]);

  const handleClick = () => {
    void navigator.clipboard.writeText(code);

    pushTransientNotification({
      status: Notification.Status.STATUS_SUCCESS,
      message: "Token copied to clipboard",
    });
  };

  return (
    <>
      <TableTitleBar label="Profile" backLink="/settings/profile" />
      <div className="thing_form">
        {createPairingTokenRes.error && (
          <InputError
            error={createPairingTokenRes.error.message || "Error creating a pairing token"}
          />
        )}
        {code && (
          <InputLabel text="token">
            <canvas ref={canvas} height="500" width="500" />
            <code
              style={{
                lineBreak: "anywhere",
                fontFamily: "monospace",
                padding: "10px",
                width: "500px",
              }}
              onClick={handleClick}
            >
              {code}
            </code>
          </InputLabel>
        )}
      </div>
    </>
  );
};

export default ProfilePairingToken;
