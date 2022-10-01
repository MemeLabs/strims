// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import QRCode from "qrcode";
import React, { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { PairingToken } from "../../../apis/strims/auth/v1/auth";
import { Notification } from "../../../apis/strims/notification/v1/notification";
import { InputError, InputLabel } from "../../../components/Form";
import InternalLink from "../../../components/InternalLink";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall } from "../../../contexts/FrontendApi";
import { useNotification } from "../../../contexts/Notification";

const ProfileConfigForm = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  const { pushTransientNotification } = useNotification();

  const [createPairingTokenRes] = useCall("replication", "createPairingToken");

  const canvas = useRef<HTMLCanvasElement>();
  useEffect(() => {
    const token = createPairingTokenRes.value?.token;
    if (token) {
      const data = PairingToken.encode(token).finish();
      console.log(data.slice());

      const code = Base64.fromUint8Array(data);
      console.log({ token, data });
      console.log({ code, length: code.length });
      // void QRCode.toCanvas(canvas.current, [{ data, mode: "byte" }]);
      void QRCode.toCanvas(canvas.current, code, { margin: 15 });
    }
  }, [createPairingTokenRes.value]);

  const handleClick = () => {
    const { token } = createPairingTokenRes.value;
    const code = Base64.fromUint8Array(PairingToken.encode(token).finish());
    void navigator.clipboard.writeText(code);

    pushTransientNotification({
      status: Notification.Status.STATUS_SUCCESS,
      message: "Token copied to clipboard",
    });
  };

  return (
    <>
      <TableTitleBar label="Profile" />
      <div className="thing_form">
        {createPairingTokenRes.error && (
          <InputError
            error={createPairingTokenRes.error.message || "Error saving debug settings"}
          />
        )}
        <InputLabel text="token">
          <InternalLink to="/settings/profile/devices">Devices</InternalLink>
          <canvas ref={canvas} height="500" width="500" />
          {createPairingTokenRes.value && (
            <code
              style={{
                lineBreak: "anywhere",
                fontFamily: "monospace",
                padding: "10px",
                width: "500px",
              }}
              onClick={handleClick}
            >
              {Base64.fromUint8Array(
                PairingToken.encode(createPairingTokenRes.value.token).finish()
              )}
            </code>
          )}
        </InputLabel>
      </div>
    </>
  );
};

export default ProfileConfigForm;
