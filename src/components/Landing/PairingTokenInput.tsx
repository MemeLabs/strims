// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./PairingTokenInput.scss";

import clsx from "clsx";
import { Html5Qrcode, Html5QrcodeSupportedFormats } from "html5-qrcode";
import { Base64 } from "js-base64";
import React, { useId, useMemo, useRef, useState } from "react";
import { Control, useController } from "react-hook-form";
import { BsExclamationCircleFill } from "react-icons/bs";
import { MdCheckCircle, MdClose } from "react-icons/md";

import { PairingToken } from "../../apis/strims/auth/v1/auth";
import { useStableCallback } from "../../hooks/useStableCallback";
import { Button, ButtonSet, InputLabel } from "../Form";
import { ProfileFormValues } from "./ProfileForm";

interface PairingTokenInputProps {
  control: Control<ProfileFormValues>;
}

const PairingTokenInput: React.FC<PairingTokenInputProps> = ({ control }) => {
  const { field } = useController({
    name: "pairingTokenString",
    defaultValue: "",
    control,
    rules: {
      validate: (v) => (!v || checkPairingToken(v) ? true : "Invalid pairing token"),
    },
  });

  const [isPasteModalOpen, setPasteModalOpen] = useState(false);

  const readerId = useId();
  const [scannerOpen, setScannerOpen] = useState(false);
  const [scanner, setScanner] = useState<Html5Qrcode>(null);
  const scanCode = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setScannerOpen(true);

    const scanner = new Html5Qrcode(readerId, {
      formatsToSupport: [Html5QrcodeSupportedFormats.QR_CODE],
      verbose: false,
    });
    setScanner(scanner);
    void scanner
      .start(
        { facingMode: "environment" },
        {
          fps: 10,
          qrbox: { width: 250, height: 250 },
          disableFlip: true,
          aspectRatio: 1,
        },
        (decodedText) => {
          void scanner.stop();
          setScannerOpen(false);
          setScanner(null);
          field.onChange(decodedText);
        },
        null
      )
      .catch(() => {
        setScannerOpen(false);
        setScanner(null);
      });
  });

  const handlePasteClick = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setPasteModalOpen(true);
  });

  const handlePasteModalDone = useStableCallback((token: string) => {
    setPasteModalOpen(false);
    field.onChange(token);
  });

  const handleLabelClick = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    if (!checkPairingToken(field.value)) {
      field.onChange("");
    }
  });

  const handleScannerCloseClick = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    void scanner.stop();
    setScannerOpen(false);
    setScanner(null);
  });

  return (
    <>
      <InputLabel text="Pairing token" inlineInput onClick={handleLabelClick}>
        {!field.value ? (
          <ButtonSet>
            <Button primary onClick={handlePasteClick}>
              Paste
            </Button>
            <Button primary onClick={scanCode}>
              Scan
            </Button>
          </ButtonSet>
        ) : checkPairingToken(field.value) ? (
          <PairingTokenValue value={field.value} />
        ) : (
          <PairingTokenError />
        )}
      </InputLabel>
      <div
        className={clsx({
          "pairing_token_input__scanner": true,
          "pairing_token_input__scanner--open": scannerOpen,
        })}
      >
        <button
          className="pairing_token_input__scanner__close_button"
          onClick={handleScannerCloseClick}
        >
          <MdClose size={32} />
        </button>
        <div id={readerId} />
      </div>
      {isPasteModalOpen && <PairingTokenInputModal onDone={handlePasteModalDone} />}
    </>
  );
};

export default PairingTokenInput;

const PairingTokenError = () => (
  <div className="pairing_token_input__result">
    Invalid token
    <BsExclamationCircleFill size={20} className="pairing_token_input__error" />
  </div>
);

interface PairingTokenValueProps {
  value: string;
}

const PairingTokenValue: React.FC<PairingTokenValueProps> = ({ value }) => {
  const name = useMemo(() => PairingToken.decode(Base64.toUint8Array(value)).profile.name, [value]);
  return (
    <div className="pairing_token_input__result">
      {name}
      <MdCheckCircle size={20} className="pairing_token_input__ok" />
    </div>
  );
};

interface PairingTokenInputModalProps {
  onDone: (token?: string) => void;
}

const PairingTokenInputModal: React.FC<PairingTokenInputModalProps> = ({ onDone }) => {
  const textarea = useRef<HTMLTextAreaElement>(null);
  const [doneDisabled, setDoneDisabled] = useState(true);

  const handleCloseClick = useStableCallback(() => onDone());

  const handleDoneClick = useStableCallback(() => onDone(textarea.current.value));

  const handleTokenChange = useStableCallback(() =>
    setDoneDisabled(!checkPairingToken(textarea.current.value))
  );

  return (
    <div className="pairing_token_input__modal__mask">
      <div className="pairing_token_input__modal__window">
        <button className="pairing_token_input__modal__close_button" onClick={handleCloseClick}>
          <MdClose size={24} />
        </button>
        <textarea
          ref={textarea}
          onChange={handleTokenChange}
          onBlur={handleTokenChange}
          className="pairing_token_input__modal__textarea"
          placeholder="Paste your pairing token."
        />
        <ButtonSet>
          <Button primary onClick={handleDoneClick} disabled={doneDisabled}>
            Done
          </Button>
        </ButtonSet>
      </div>
    </div>
  );
};

const checkPairingToken = (token: string) => {
  if (!token) {
    return false;
  }
  try {
    PairingToken.decode(Base64.toUint8Array(token));
    return true;
  } catch {
    return false;
  }
};
