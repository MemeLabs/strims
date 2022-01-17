import "./InputLabel.scss";

import clsx from "clsx";
import React, { ReactHTML } from "react";

export interface InputLabelProps {
  required?: boolean;
  text: string;
  description?: string;
  inputType?: string;
  component?: keyof ReactHTML;
}

const InputLabel: React.FC<InputLabelProps> = ({
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

export default InputLabel;