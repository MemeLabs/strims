// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./PairingTokenInput.scss";

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { CSSProperties, useEffect, useMemo, useRef, useState } from "react";
import { Control, useController } from "react-hook-form";
import { BsExclamationCircleFill } from "react-icons/bs";
import { MdCheckCircle, MdClose } from "react-icons/md";

import { PairingToken } from "../../apis/strims/auth/v1/auth";
import { useStableCallback } from "../../hooks/useStableCallback";
import { qrDecoder } from "../../lib/qr";
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

  const [scannerOpen, setScannerOpen] = useState(false);
  const scanCode = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setScannerOpen(true);
  });

  const handleScan = useStableCallback((token: string) => {
    setScannerOpen(false);
    field.onChange(token);
  });

  const closeScanner = useStableCallback(() => setScannerOpen(false));

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
    closeScanner();
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
        {scannerOpen && <QRScanner onScan={handleScan} onError={closeScanner} />}
      </div>
      {isPasteModalOpen && <PairingTokenInputModal onDone={handlePasteModalDone} />}
    </>
  );
};

export default PairingTokenInput;

// The scanner's picture is this wide, and its middle's square this tall is
// read for a QR code this many times a second.
const SCANNER_WIDTH = 300;
const SCAN_BOX = 250;
const SCANS_PER_SECOND = 10;

// The marks at the scanned square's corners, 5 px outside it.
const BOX_CORNERS: CSSProperties[] = [
  { width: 40, height: 5, top: -5, left: 0 },
  { width: 40, height: 5, top: -5, right: 0 },
  { width: 40, height: 5, bottom: -5, left: 0 },
  { width: 40, height: 5, bottom: -5, right: 0 },
  { width: 5, height: 45, top: -5, left: -5 },
  { width: 5, height: 45, bottom: -5, left: -5 },
  { width: 5, height: 45, top: -5, right: -5 },
  { width: 5, height: 45, bottom: -5, right: -5 },
];

interface QRScannerProps {
  onScan: (text: string) => void;
  onError: () => void;
}

// The rear camera's picture, its middle square read for a QR code until one
// reads (onScan) and the rest shaded. The camera stops when it unmounts;
// onError when there's no camera or no permission to use it.
const QRScanner: React.FC<QRScannerProps> = ({ onScan, onError }) => {
  const video = useRef<HTMLVideoElement>(null);
  const [size, setSize] = useState<{ width: number; height: number }>(null);
  const handleScan = useStableCallback(onScan);
  const handleError = useStableCallback(onError);

  useEffect(() => {
    let live = true;
    let stream: MediaStream = null;
    let timer = 0;
    const stop = () => {
      live = false;
      window.clearTimeout(timer);
      stream?.getTracks().forEach((t) => t.stop());
    };
    const fail = () => {
      if (live) {
        stop();
        handleError();
      }
    };

    const start = async () => {
      const decode = await qrDecoder();
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment", aspectRatio: 1 },
        audio: false,
      });
      const v = video.current;
      if (!live || !v) {
        stop();
        return;
      }
      v.srcObject = stream;
      await v.play();
      const { clientWidth: width, clientHeight: height } = v;
      const box = Math.min(SCAN_BOX, width, height);
      setSize({ width, height });

      const canvas = document.createElement("canvas");
      canvas.width = box;
      canvas.height = box;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      const sx = v.videoWidth / width;
      const sy = v.videoHeight / height;
      const scan = async () => {
        context.drawImage(
          v,
          ((width - box) / 2) * sx,
          ((height - box) / 2) * sy,
          box * sx,
          box * sy,
          0,
          0,
          box,
          box
        );
        const text = await decode(canvas);
        if (!live) {
          return;
        }
        if (text !== null) {
          stop();
          handleScan(text);
          return;
        }
        timer = window.setTimeout(() => scan().catch(fail), 1000 / SCANS_PER_SECOND);
      };
      await scan();
    };
    start().catch(fail);
    return stop;
  }, []);

  const box = size && Math.min(SCAN_BOX, size.width, size.height);
  return (
    <div className="pairing_token_input__reader" style={{ width: SCANNER_WIDTH }}>
      <video ref={video} className="pairing_token_input__reader__video" muted playsInline />
      {size && (
        <div
          className="pairing_token_input__reader__shade"
          style={{
            borderLeftWidth: (size.width - box) / 2,
            borderRightWidth: (size.width - box) / 2,
            borderTopWidth: (size.height - box) / 2,
            borderBottomWidth: (size.height - box) / 2,
          }}
        >
          {BOX_CORNERS.map((style, i) => (
            <div key={i} className="pairing_token_input__reader__corner" style={style} />
          ))}
        </div>
      )}
    </div>
  );
};

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
