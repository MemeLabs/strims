import clsx from "clsx";
import { get } from "lodash";
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
    <div className={labelClass}>
      <div className="input_label__text">{text}</div>
      <div className="input_label__body">
        {children}
        {description && <div className="input_label__description">{description}</div>}
      </div>
    </div>
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
    field: { value, ...field },
    fieldState: { error },
  } = useController({ defaultValue: false, ...controllerProps });

  return (
    <InputLabel text={label} description={description}>
      <input
        {...field}
        checked={value}
        className="input input_toggle"
        type="checkbox"
        disabled={disabled}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface ImageValue {
  data: Uint8Array;
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

export const ImageInput = <T extends FieldValues>({
  maxSize = 512 * 1024,
  classNameBase = "input_image",
  placeholder: Placeholder = ImageInputPlaceholder,
  preview: Preview = ImageInputPreview,
  ...controllerProps
}: ImageInput & CompatibleUseControllerProps<T, ImageValue>): ReactElement => {
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
        data,
        type: file.type,
        height: img.height,
        width: img.width,
      });
  };

  React.useEffect(() => {
    const defaultValue = get(
      controllerProps.control.defaultValuesRef.current,
      controllerProps.name
    ) as ImageValue;
    if (defaultValue) {
      const { data, type } = defaultValue;
      setPreviewUrl(URL.createObjectURL(new Blob([data], { type })));
    }
  }, []);

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
