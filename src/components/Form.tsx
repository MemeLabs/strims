import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { ReactElement, ReactHTML } from "react";
import Dropzone from "react-dropzone";
import { FieldError, FieldValues, Path, UseControllerProps, useController } from "react-hook-form";
import { FiAlertTriangle } from "react-icons/fi";
import { MdAddAPhoto } from "react-icons/md";
import Select from "react-select";
import CreatableSelect from "react-select/creatable";

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
  inputType?: string;
  component?: keyof ReactHTML;
}

export const InputLabel: React.FC<InputLabelProps> = ({
  children,
  required,
  text,
  description,
  inputType = "default",
  component = "label",
}) => {
  React.createElement;
  const labelClass = clsx({
    "input_label": true,
    "input_label--required": required,
    [`input_label--${inputType}`]: true,
  });

  return React.createElement(component, {
    className: labelClass,
    children: (
      <>
        <div className="input_label__text">{text}</div>
        <div className="input_label__body">
          {children}
          {description && <div className="input_label__description">{description}</div>}
        </div>
      </>
    ),
  });
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
  placeholder?: string;
  type?: "text" | "password" | "number";
  format?: "text";
  disabled?: boolean;
}

export const TextInput = <T extends FieldValues>({
  label,
  description,
  placeholder,
  type: type,
  disabled,
  ...controllerProps
}: TextInputProps & CompatibleUseControllerProps<T, string | number>): ReactElement => {
  const defaultValue = type === "number" ? 0 : "";
  const {
    field,
    fieldState: { error },
  } = useController({
    // @ts-ignore
    defaultValue,
    ...controllerProps,
  });

  return (
    <InputLabel
      required={isRequired(controllerProps)}
      text={label}
      description={description}
      inputType="text"
    >
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
  placeholder?: string;
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
  } = useController({
    // @ts-ignore
    defaultValue: "",
    ...controllerProps,
  });

  return (
    <InputLabel
      required={isRequired(controllerProps)}
      text={label}
      description={description}
      inputType="textarea"
    >
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
  } = useController({
    // @ts-ignore
    defaultValue: false,
    ...controllerProps,
  });

  return (
    <InputLabel text={label} description={description} inputType="toggle">
      <input
        {...field}
        checked={value}
        className="input input_toggle"
        type="checkbox"
        disabled={disabled}
      />
      <div className="input_toggle__switch">
        <div className="input_toggle__switch__track" />
      </div>
      <InputError error={error} />
    </InputLabel>
  );
};

export interface SelectOption<T> {
  label: string;
  value: T;
}

export interface SelectInputProps<T, M> {
  label: string;
  description?: string;
  placeholder?: string;
  isMulti?: M;
  disabled?: boolean;
  options: T[];
}

export const SelectInput = <T extends FieldValues, F extends SelectOption<any>, M extends boolean>({
  label,
  description,
  placeholder,
  isMulti = false as M,
  disabled,
  options,
  ...controllerProps
}: SelectInputProps<F, M> &
  CompatibleUseControllerProps<T, M extends true ? F[] : F>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController(controllerProps);

  return (
    <InputLabel text={label} description={description} inputType="select">
      <Select
        {...field}
        className="input_select"
        classNamePrefix="react_select"
        placeholder={placeholder}
        // @ts-ignore
        options={options}
        isMulti={isMulti}
        disabled={disabled}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface CreatableSelectInputProps {
  label: string;
  description?: string;
  placeholder?: string;
  disabled?: boolean;
}

export const CreatableSelectInput = <T extends FieldValues>({
  label,
  description,
  placeholder,
  disabled,
  ...controllerProps
}: CreatableSelectInputProps &
  CompatibleUseControllerProps<T, SelectOption<string>[]>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController(controllerProps);

  return (
    <InputLabel text={label} description={description} inputType="select">
      <CreatableSelect
        {...field}
        isMulti={true}
        className="input_select"
        classNamePrefix="react_select"
        placeholder={placeholder}
        disabled={disabled}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

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
