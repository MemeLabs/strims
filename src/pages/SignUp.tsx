import * as React from "react";
import { useForm } from "react-hook-form";
import { Link, Redirect } from "react-router-dom";

import { ICreateProfileRequest } from "../apis/strims/profile/v1/profile";
import { InputError, TextInput } from "../components/Form";
import LandingPageLayout from "../components/LandingPageLayout";
import { useCall } from "../contexts/Api";
import { useProfile } from "../contexts/Profile";

const SignUpPage = () => {
  const [getProfilesRes] = useCall("profile", "list");
  const [{ profile, error, loading }, profileActions] = useProfile();
  const isLocalAccountsEmpty = !getProfilesRes.loading && !getProfilesRes.value?.profiles.length;
  const { register, handleSubmit, errors } = useForm<ICreateProfileRequest>({
    mode: "onBlur",
  });

  React.useEffect(profileActions.clearError, []);

  if (profile) {
    return <Redirect to="/" />;
  }

  const onSubmit = handleSubmit((data) => profileActions.createProfile(data));

  return (
    <LandingPageLayout>
      <form onSubmit={onSubmit}>
        {getProfilesRes.error && (
          <InputError error={getProfilesRes.error.message || "Error loading profiles"} />
        )}
        {error && <InputError error={error.message || "Error creating profile"} />}
        <TextInput
          error={errors.name}
          inputRef={register({
            required: {
              value: true,
              message: "Name is required",
            },
            pattern: {
              value: /^\S+$/i,
              message: "Name contains invalid characters",
            },
          })}
          label="Profile Name"
          name="name"
          placeholder="Enter a profile name"
          required
        />
        <TextInput
          error={errors.password}
          inputRef={register({
            required: {
              value: true,
              message: "Password is required",
            },
          })}
          label="Password"
          name="password"
          placeholder="Enter a password"
          required
          type="password"
        />
        <div className="input_buttons">
          {!isLocalAccountsEmpty && (
            <Link className="input input_button input_button--borderless" to="/login">
              Log in
            </Link>
          )}
          <button className="input input_button" disabled={loading}>
            Create Profile
          </button>
        </div>
      </form>
    </LandingPageLayout>
  );
};

export default SignUpPage;
