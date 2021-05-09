import clsx from "clsx";
import React, { ReactElement } from "react";
import Dropzone from "react-dropzone";
import { FieldError, FieldValues, UseControllerProps, useController } from "react-hook-form";
import { FiAlertTriangle } from "react-icons/fi";
import { MdAddAPhoto } from "react-icons/md";

type CompatibleFieldPath<T extends FieldValues, V> = {
  [K in keyof T]: T[K] extends V ? K : never;
}[keyof T];
type CompatibleUseControllerProps<T, V> = UseControllerProps<T> & {
  name: CompatibleFieldPath<T, V>;
};

const isRequired = <T extends FieldValues>({ rules }: UseControllerProps<T>) =>
  Boolean(rules?.required);

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
  error: FieldError | Error | string;
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
  label: string;
  description?: string;
  placeholder: string;
  type?: "text" | "password";
  disabled?: boolean;
}

export const TextInput = <T extends FieldValues>({
  label,
  description,
  placeholder,
  type,
  disabled,
  ...controllerProps
}: TextInputProps & CompatibleUseControllerProps<T, string>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({ defaultValue: "", ...controllerProps });

  return (
    <InputLabel required={isRequired(controllerProps)} text={label} description={description}>
      <input
        {...field}
        className="input input_text"
        placeholder={placeholder}
        type={type}
        disabled={disabled}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface TextAreaInputProps {
  label: string;
  description?: string;
  name: string;
  placeholder: string;
  disabled?: boolean;
}

export const TextAreaInput = <T extends FieldValues>({
  label,
  description,
  placeholder,
  disabled,
  ...controllerProps
}: TextAreaInputProps & CompatibleUseControllerProps<T, string>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({ defaultValue: "", ...controllerProps });

  return (
    <InputLabel required={isRequired(controllerProps)} text={label} description={description}>
      <textarea
        {...field}
        className="input input_textarea"
        placeholder={placeholder}
        disabled={disabled}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface ToggleInputProps {
  label: string;
  description?: string;
  disabled?: boolean;
}

export const ToggleInput = <T extends FieldValues>({
  label,
  description,
  disabled,
  ...controllerProps
}: ToggleInputProps & CompatibleUseControllerProps<T, boolean>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({ defaultValue: false, ...controllerProps });

  return (
    <InputLabel text={label} description={description}>
      <input {...field} className="input input_toggle" type="checkbox" disabled={disabled} />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface ImageValue {
  data: Uint8Array;
  type: string;
}

export interface AvatarInput {
  maxSize?: number;
}

export const AvatarInput = <T extends FieldValues>({
  maxSize = 512 * 1024,
  ...controllerProps
}: AvatarInput & CompatibleUseControllerProps<T, ImageValue>): ReactElement => {
  const [previewUrl, setPreviewUrl] = React.useState<string>();
  React.useEffect(() => () => URL.revokeObjectURL(previewUrl), [previewUrl]);

  const {
    field: { onChange },
    fieldState: { error },
  } = useController(controllerProps);

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
    <>
      <InputError error={error} />
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
    </>
  );
};
