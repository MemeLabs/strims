import React from "react";
import { useForm } from "react-hook-form";
import { FiUser, FiUserPlus } from "react-icons/fi";
import { Link, Redirect, useLocation } from "react-router-dom";

import { ILoadProfileRequest, IProfileSummary } from "../apis/strims/profile/v1/profile";
import { InputError, TextInput } from "../components/Form";
import LandingPageLayout from "../components/LandingPageLayout";
import { useCall } from "../contexts/FrontendApi";
import { useProfile } from "../contexts/Profile";
import useQuery from "../hooks/useQuery";

const VALID_NEXT_PATH = /^\/\w[\w/_\-.?=#%&]*$/;

interface LoginQueryParams {
  next: string;
}

const LoginPage: React.FC = () => {
  const [listProfilesRes] = useCall("profile", "list");
  const [{ profile, error, loading }, profileActions] = useProfile();
  const [selectedProfile, setSelectedProfile] = React.useState<IProfileSummary | null>(null);
  const { register, handleSubmit, errors } = useForm<ILoadProfileRequest>({
    mode: "onBlur",
  });
  const { next } = useQuery<LoginQueryParams>(useLocation().search);

  React.useEffect(profileActions.clearError, []);

  if (!listProfilesRes.loading && !listProfilesRes.value?.profiles.length) {
    return <Redirect to="/signup" />;
  }
  if (profile) {
    return <Redirect to={VALID_NEXT_PATH.test(next) ? next : "/"} />;
  }

  if (!selectedProfile) {
    return (
      <LandingPageLayout>
        <div className="login_profile_list">
          {listProfilesRes.value?.profiles.map((summary) => (
            <div
              className="login_profile_list__item"
              key={summary.id.toString()}
              onClick={() => setSelectedProfile(summary)}
            >
              <FiUser className="login_profile_list__icon" />
              <span className="login_profile_list__text">{summary.name}</span>
            </div>
          ))}
          <Link className="login_profile_list__item" to="/signup">
            <FiUserPlus className="login_profile_list__icon" />
            <span className="login_profile_list__text">Create Profile</span>
          </Link>
        </div>
      </LandingPageLayout>
    );
  }

  const onSubmit = handleSubmit((data) =>
    profileActions.loadProfile({
      id: selectedProfile.id,
      ...data,
    })
  );

  return (
    <LandingPageLayout>
      <form onSubmit={onSubmit}>
        {listProfilesRes.error && (
          <InputError error={listProfilesRes.error.message || "Error loading profiles"} />
        )}
        {error && <InputError error={error.message || "Error logging in"} />}
        <TextInput
          error={errors.name && "Name is required"}
          inputRef={register({ required: true })}
          label="Profile Name"
          name="name"
          placeholder="Enter your profile name"
          required
          defaultValue={selectedProfile.name}
        />
        <TextInput
          error={errors.password && "Password is required"}
          inputRef={(ref) => {
            if (ref) {
              ref.focus();
            }
            register({ required: true })(ref);
          }}
          label="Password"
          name="password"
          placeholder="Enter your password"
          required
          type="password"
        />
        <div className="input_buttons">
          <Link className="input input_button input_button--borderless" to="/signup">
            Create Profile
          </Link>
          <button className="input input_button" disabled={loading}>
            Load Profile
          </button>
        </div>
      </form>
    </LandingPageLayout>
  );
};

export default LoginPage;
