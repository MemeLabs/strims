// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import {
  BinaryBitmap,
  ChecksumException,
  FormatException,
  HybridBinarizer,
  NotFoundException,
  QRCodeReader,
  RGBLuminanceSource,
} from "@zxing/library";

// RGBA pixels, as ImageData holds them.
export interface Pixels {
  data: Uint8ClampedArray;
  width: number;
  height: number;
}

// The text of the QR code in an image, or null where there's none (or none
// that reads). zxing's reader: the scanner's decoder where the browser has no
// BarcodeDetector for QR codes, desktop Chromium's and Firefox's among them.
export const decodeQR = ({ data, width, height }: Pixels): string | null => {
  const luminances = new Uint8ClampedArray(width * height);
  for (let i = 0, j = 0; i < luminances.length; i++, j += 4) {
    luminances[i] = (data[j] * 306 + data[j + 1] * 601 + data[j + 2] * 117) >> 10;
  }
  const bitmap = new BinaryBitmap(
    new HybridBinarizer(new RGBLuminanceSource(luminances, width, height))
  );
  try {
    return new QRCodeReader().decode(bitmap).getText();
  } catch (e) {
    if (
      e instanceof NotFoundException ||
      e instanceof ChecksumException ||
      e instanceof FormatException
    ) {
      return null;
    }
    throw e;
  }
};

// The Barcode Detection API, where the browser has it (Chromium on Android,
// macOS and ChromeOS, Safari).
interface BarcodeDetectorLike {
  detect(image: CanvasImageSource): Promise<{ rawValue: string }[]>;
}

interface BarcodeDetectorClass {
  new (options: { formats: string[] }): BarcodeDetectorLike;
  getSupportedFormats(): Promise<string[]>;
}

// Reads a canvas's QR code: its text, or null.
export type QRDecoder = (canvas: HTMLCanvasElement) => Promise<string | null>;

// The browser's BarcodeDetector where it reads QR codes, else decodeQR over
// the canvas's pixels.
export const qrDecoder = async (): Promise<QRDecoder> => {
  const Detector = (globalThis as { BarcodeDetector?: BarcodeDetectorClass }).BarcodeDetector;
  if (Detector && (await Detector.getSupportedFormats()).includes("qr_code")) {
    const detector = new Detector({ formats: ["qr_code"] });
    return async (canvas) => (await detector.detect(canvas))[0]?.rawValue ?? null;
  }
  return (canvas) =>
    Promise.resolve(
      decodeQR(canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height))
    );
};
