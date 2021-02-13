import clsx from "clsx";
import React from "react";
import Dropzone from "react-dropzone";
import { Control, Controller } from "react-hook-form";
import { FiAlertTriangle } from "react-icons/fi";
import { MdAddAPhoto } from "react-icons/md";

export interface InputLabelProps {
  required?: boolean;
  text: string;
  description?: string;
}

export const InputLabel: React.FC<InputLabelProps> = ({
  children,
  required,
  text,
  description,
}) => {
  const labelClass = clsx({
    "input_label": true,
    "input_label--required": required,
  });

  return (
    <label className={labelClass}>
      <div className="input_label__text">{text}</div>
      <div className="input_label__body">
        {children}
        {description && <div className="input_label__description">{description}</div>}
      </div>
    </label>
  );
};

export interface InputErrorProps {
  error: Error | string;
}

export const InputError: React.FC<InputErrorProps> = ({ error }) => {
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

export interface TextInputProps {
  error?: Error | string;
  inputRef: React.Ref<HTMLInputElement>;
  label: string;
  description?: string;
  name: string;
  placeholder: string;
  required?: boolean;
  type?: "text" | "password";
  disabled?: boolean;
  defaultValue?: string;
}

export const TextInput: React.FC<TextInputProps> = ({
  error,
  inputRef,
  label,
  description,
  name,
  placeholder,
  required,
  type,
  disabled,
  defaultValue,
}) => (
  <InputLabel required={required} text={label} description={description}>
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

export interface TextAreaInputProps {
  error?: Error | string;
  inputRef: React.Ref<HTMLTextAreaElement>;
  label: string;
  description?: string;
  name: string;
  placeholder: string;
  required?: boolean;
  disabled?: boolean;
  defaultValue?: string;
}

export const TextAreaInput: React.FC<TextAreaInputProps> = ({
  error,
  inputRef,
  label,
  description,
  name,
  placeholder,
  required,
  disabled,
  defaultValue,
}) => (
  <InputLabel required={required} text={label} description={description}>
    <textarea
      className="input input_textarea"
      name={name}
      placeholder={placeholder}
      ref={inputRef}
      disabled={disabled}
      defaultValue={defaultValue}
    />
    <InputError error={error} />
  </InputLabel>
);

export interface ToggleInputProps {
  error?: Error | string;
  inputRef: React.Ref<HTMLInputElement>;
  label: string;
  description?: string;
  name: string;
  disabled?: boolean;
  defaultValue?: boolean;
}

export const ToggleInput: React.FC<ToggleInputProps> = ({
  error,
  inputRef,
  label,
  description,
  name,
  disabled,
  defaultValue,
}) => (
  <InputLabel text={label} description={description}>
    <input
      className="input input_toggle"
      name={name}
      ref={inputRef}
      type="checkbox"
      disabled={disabled}
      defaultChecked={defaultValue}
    />
    <InputError error={error} />
  </InputLabel>
);

export interface ImageValue {
  data: Uint8Array;
  type: string;
}

export interface AvatarInput {
  name: string;
  control: Control<Record<string, any>>;
  maxSize?: number;
}

export const AvatarInput: React.FC<AvatarInput> = ({ name, control, maxSize = 512 * 1024 }) => {
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
