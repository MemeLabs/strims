import clsx from "clsx";
import * as React from "react";
import Dropzone from "react-dropzone";
import { Control, Controller } from "react-hook-form";
import { FiAlertTriangle } from "react-icons/fi";
import { MdAddAPhoto } from "react-icons/md";

export const InputLabel = ({
  children,
  required,
  text,
}: {
  children: any;
  required: boolean;
  text: string;
}) => {
  const labelClass = clsx({
    "input_label": true,
    "input_label--required": required,
  });

  return (
    <label className={labelClass}>
      <span className="input_label__text">{text}</span>
      {children}
    </label>
  );
};

export const InputError = ({ error }: { error: any }) => {
  if (!error) {
    return null;
  }

  let message = "Invalid value";
  if (typeof error === "string") {
    message = error;
  } else if (error.message) {
    message = error.message;
  }

  return (
    <span className="input_error">
      <FiAlertTriangle className="input_error__icon" />
      <span className="input_error__text">{message}</span>
    </span>
  );
};

export const TextInput = ({
  error,
  inputRef,
  label,
  name,
  placeholder,
  required,
  type,
  disabled,
  defaultValue,
}: {
  error?: any;
  inputRef: React.Ref<HTMLInputElement>;
  label: string;
  name: string;
  placeholder: string;
  required?: boolean;
  type?: "text" | "password";
  disabled?: boolean;
  defaultValue?: string;
}) => {
  return (
    <InputLabel required={required} text={label}>
      <input
        className="input input_text"
        name={name}
        placeholder={placeholder}
        ref={inputRef}
        type={type}
        disabled={disabled}
        defaultValue={defaultValue}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface ImageValue {
  data: Uint8Array;
  type: string;
}

export const AvatarInput = ({
  name,
  control,
  maxSize = 512 * 1024,
}: {
  name: string;
  control: Control<Record<string, any>>;
  maxSize?: number;
}) => {
  const [previewUrl, setPreviewUrl] = React.useState<string>();
  React.useEffect(() => () => URL.revokeObjectURL(previewUrl), [previewUrl]);

  return (
    <Controller
      name={name}
      control={control}
      defaultValue=""
      render={({ onChange }) => {
        const handleDrop = async ([file]: File[]) => {
          if (!file) {
            onChange(null);
            return;
          }

          setPreviewUrl(URL.createObjectURL(file));

          const data = await new Promise<ArrayBuffer>((resolve, reject) => {
            const reader = new FileReader();
            reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
            reader.onerror = () => reject();
            reader.readAsArrayBuffer(file);
          });
          onChange({
            type: file.type,
            data,
          });
        };

        return (
          <Dropzone maxSize={maxSize} multiple={false} accept="image/*" onDrop={handleDrop}>
            {({ getRootProps, getInputProps }) => (
              <div {...getRootProps()} className="input_avatar">
                {previewUrl ? (
                  <img src={previewUrl} className="input_avatar__preview" />
                ) : (
                  <div className="input_avatar__placeholder">
                    <MdAddAPhoto size={30} className="input_avatar__placeholder__icon" />
                    <span className="input_avatar__placeholder__text">upload</span>
                  </div>
                )}
                <input name="file" {...getInputProps()} />
              </div>
            )}
          </Dropzone>
        );
      }}
    />
  );
};
