import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { ComponentProps, ReactElement, ReactHTML } from "react";
import Dropzone from "react-dropzone";
import { FieldError, FieldValues, UseControllerProps, useController } from "react-hook-form";
import { FiAlertTriangle } from "react-icons/fi";
import { MdAddAPhoto } from "react-icons/md";
import Select, { Props as SelectProps } from "react-select";
import CreatableSelect from "react-select/creatable";

import { useLayout } from "../contexts/Layout";

type CompatibleFieldPath<T extends FieldValues, V> = {
  [K in keyof T]: T[K] extends V ? K : never;
}[keyof T];
type CompatibleUseControllerProps<T, V> = UseControllerProps<T> & {
  name: CompatibleFieldPath<T, V>;
};

const isRequired = <T extends FieldValues>(rules: UseControllerProps<T>["rules"]) => {
  switch (typeof rules?.required) {
    case "undefined":
      return false;
    case "boolean":
      return rules.required;
    case "string":
      return true;
    default:
      return rules?.required?.value;
  }
};

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

export interface TextInputProps extends ComponentProps<"input"> {
  label: string;
  description?: string;
  type?: "text" | "password" | "number";
  format?: "text";
}

export const TextInput = <T extends FieldValues>({
  label,
  description,
  type,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: TextInputProps & CompatibleUseControllerProps<T, string | number>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    // @ts-ignore
    defaultValue: defaultValue || (type === "number" ? 0 : ""),
    control,
  });

  return (
    <InputLabel
      required={isRequired(rules)}
      text={label}
      description={description}
      inputType="text"
    >
      <input
        {...inputProps}
        {...field}
        onChange={(e) => {
          field.onChange(type === "number" ? parseFloat(e.target.value) : e.target.value);
          inputProps.onChange?.(e);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        type={type}
        className={clsx(className, "input", "input_text")}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface TextAreaInputProps extends ComponentProps<"textarea"> {
  label: string;
  description?: string;
}

export const TextAreaInput = <T extends FieldValues>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: TextAreaInputProps & CompatibleUseControllerProps<T, string>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    // @ts-ignore
    defaultValue: defaultValue || "",
    control,
  });

  return (
    <InputLabel
      required={isRequired(rules)}
      text={label}
      description={description}
      inputType="textarea"
    >
      <textarea
        {...inputProps}
        {...field}
        onChange={(e) => {
          field.onChange(e);
          inputProps.onChange?.(e);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        className={clsx(className, "input", "input_textarea")}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface ToggleInputProps extends ComponentProps<"input"> {
  label: string;
  description?: string;
}

export const ToggleInput = <T extends FieldValues>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: ToggleInputProps & CompatibleUseControllerProps<T, boolean>): ReactElement => {
  const {
    field: { value, ...field },
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    // @ts-ignore
    defaultValue: defaultValue || false,
    control,
  });

  return (
    <InputLabel text={label} description={description} inputType="toggle">
      <input
        {...inputProps}
        {...field}
        onChange={(e) => {
          field.onChange(e);
          inputProps.onChange?.(e);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        checked={value}
        className="input input_toggle"
        type="checkbox"
      />
      <div className={clsx(className, "input_toggle__switch")}>
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

export interface SelectInputProps<T extends SelectOption<any>, M extends boolean>
  extends SelectProps<T, M> {
  label: string;
  description?: string;
}

export const SelectInput = <T extends FieldValues, F extends SelectOption<any>, M extends boolean>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: SelectInputProps<F, M> &
  CompatibleUseControllerProps<T, M extends true ? F[] : F>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    defaultValue,
    control,
  });

  const { root } = useLayout();

  return (
    <InputLabel
      required={isRequired(rules)}
      text={label}
      description={description}
      inputType="select"
    >
      <Select
        classNamePrefix="input_select"
        menuPortalTarget={root.current}
        menuPlacement="auto"
        {...(inputProps as unknown)}
        {...field}
        onChange={(value, action) => {
          field.onChange(value);
          inputProps.onChange?.(value, action);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        className={clsx(className, "input_select")}
      />
      <InputError error={error} />
    </InputLabel>
  );
};

export interface CreatableSelectInputProps<T> extends SelectProps<T, true> {
  label: string;
  description?: string;
}

export const CreatableSelectInput = <T extends FieldValues>({
  label,
  description,
  className,
  name,
  rules,
  shouldUnregister,
  defaultValue,
  control,
  ...inputProps
}: CreatableSelectInputProps<T> &
  CompatibleUseControllerProps<T, SelectOption<string>[]>): ReactElement => {
  const {
    field,
    fieldState: { error },
  } = useController({
    name,
    rules,
    shouldUnregister,
    defaultValue,
    control,
  });

  const { root } = useLayout();

  return (
    <InputLabel
      required={isRequired(rules)}
      text={label}
      description={description}
      inputType="select"
    >
      <CreatableSelect
        classNamePrefix="input_select"
        menuPortalTarget={root.current}
        menuPlacement="auto"
        {...(inputProps as unknown)}
        {...field}
        onChange={(value, action) => {
          field.onChange(value);
          inputProps.onChange?.(value, action);
        }}
        onBlur={(e) => {
          field.onBlur();
          inputProps.onBlur?.(e);
        }}
        isMulti={true}
        className={clsx(className, "input_select")}
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
