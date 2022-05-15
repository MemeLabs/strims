// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React from "react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";
import createUrlRegexp from "url-regex-safe";

import { InputError, TextInput, ToggleInput } from "../Form";

export interface ProfileFormValues {
  name: string;
  password?: string;
  advanced: boolean;
  serverAddress?: string;
  unencrypted: boolean;
  persistLogin: boolean;
}

interface ProfileFormProps {
  onSubmit: (values: ProfileFormValues) => void;
  error?: string;
  secondaryVisible?: boolean;
  secondaryUri?: string;
  secondaryLabel?: string;
  submitLabel: string;
  defaultValues?: Partial<ProfileFormValues>;
}

const ProfileForm: React.FC<ProfileFormProps> = ({
  onSubmit,
  error,
  secondaryVisible = true,
  secondaryUri,
  secondaryLabel,
  submitLabel,
  defaultValues,
}) => {
  const { control, handleSubmit, watch } = useForm<ProfileFormValues>({
    mode: "onBlur",
    defaultValues,
  });
  const advanced = watch("advanced");

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error} />}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Name is required",
          },
          pattern: {
            value: /^\S+$/i,
            message: "Name contains invalid characters",
          },
        }}
        label="Profile Name"
        name="name"
        placeholder="Enter a profile name"
      />
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Password is required",
          },
        }}
        label="Password"
        name="password"
        placeholder="Enter a password"
        type="password"
        autoComplete="password"
      />
      <div
        className={clsx({
          "landing_page__accordion": true,
          "landing_page__accordion--enabled": advanced,
        })}
      >
        <ToggleInput control={control} label="Advanced" name="advanced" />

        <div className="landing_page__accordion__content">
          <TextInput
            control={control}
            rules={{
              pattern: {
                value: createUrlRegexp(),
                message: "Invalid url",
              },
            }}
            label="Server address"
            name="serverAddress"
            placeholder="wss://next.strims.gg"
            // defaultValue={`wss://${location.host}/api`}
            // defaultValue={`wss://${location.host}/manage`}
          />
          <ToggleInput control={control} label="Stay logged in" name="persistLogin" />
        </div>
      </div>
      <div className="input_buttons">
        {secondaryVisible && (
          <Link className="input input_button input_button--borderless" to={secondaryUri}>
            {secondaryLabel}
          </Link>
        )}
        <button className="input input_button">{submitLabel}</button>
      </div>
    </form>
  );
};

export default ProfileForm;
