// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ImageInput.scss";

import { Base64 } from "js-base64";
import React, { ReactElement } from "react";
import Dropzone from "react-dropzone";
import { FieldValues, useController } from "react-hook-form";
import { MdAddAPhoto } from "react-icons/md";

import { CompatibleUseControllerProps } from "./Form";
import InputError from "./InputError";

export interface ImageValue {
  // react-hook-form schema cannot contain recursive types (Uint8Array) so we
  // have to base64 encode binary data to use it in forms.
  data: string;
  type: string;
  height: number;
  width: number;
}

export interface ImageInputPlaceholderProps {
  classNameBase: string;
}

export const ImageInputPlaceholder: React.FC<ImageInputPlaceholderProps> = ({ classNameBase }) => (
  <div className={`${classNameBase}__placeholder`}>
    <MdAddAPhoto size={30} className={`${classNameBase}__placeholder__icon`} />
    <span className={`${classNameBase}__placeholder__text`}>upload</span>
  </div>
);

export interface ImageInputPreviewProps {
  src: string;
  classNameBase: string;
}

export const ImageInputPreview: React.FC<ImageInputPreviewProps> = ({ src, classNameBase }) => (
  <img src={src} className={`${classNameBase}__preview`} />
);

export interface ImageInput {
  maxSize?: number;
  classNameBase?: string;
  placeholder?: React.ComponentType<ImageInputPlaceholderProps>;
  preview?: React.ComponentType<ImageInputPreviewProps>;
}

const ImageInput = <T extends FieldValues>({
  maxSize = 512 * 1024,
  classNameBase = "input_image",
  placeholder: Placeholder = ImageInputPlaceholder,
  preview: Preview = ImageInputPreview,
  ...controllerProps
}: ImageInput & CompatibleUseControllerProps<T, ImageValue>): ReactElement => {
  const [previewUrl, setPreviewUrl] = React.useState<string>();
  React.useEffect(() => () => URL.revokeObjectURL(previewUrl), [previewUrl]);

  const {
    field: { onChange, value },
    fieldState: { error },
  } = useController(controllerProps);

  React.useEffect(() => {
    if (value) {
      const { data, type } = value as ImageValue;
      setPreviewUrl(URL.createObjectURL(new Blob([Base64.toUint8Array(data)], { type })));
    }
  }, []);

  const handleDrop = async ([file]: File[]) => {
    if (!file) {
      onChange(null);
      return;
    }

    const url = URL.createObjectURL(file);
    setPreviewUrl(url);

    const data = await new Promise<ArrayBuffer>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
      reader.onerror = () => reject();
      reader.readAsArrayBuffer(file);
    });

    const img = new Image();
    img.src = url;
    img.onload = () =>
      onChange({
        data: Base64.fromUint8Array(new Uint8Array(data)),
        type: file.type,
        height: img.height,
        width: img.width,
      });
  };

  return (
    <>
      <InputError error={error} />
      <Dropzone maxSize={maxSize} multiple={false} accept="image/*" onDrop={handleDrop}>
        {({ getRootProps, getInputProps }) => (
          <div {...getRootProps()} className={classNameBase}>
            {previewUrl ? (
              <Preview src={previewUrl} classNameBase={classNameBase} />
            ) : (
              <Placeholder classNameBase={classNameBase} />
            )}
            <input name="file" {...getInputProps()} />
          </div>
        )}
      </Dropzone>
    </>
  );
};

export default ImageInput;
