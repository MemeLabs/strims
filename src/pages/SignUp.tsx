import React from "react";
import { Navigate } from "react-router-dom";

import ProfileForm, { ProfileFormValues } from "../components/Landing/ProfileForm";
import LandingPageLayout from "../components/LandingPageLayout";
import { useSession } from "../contexts/Session";

const SignUpPage: React.FC = () => {
  const [session, sessionOps] = useSession();

  if (session.profile) {
    return <Navigate to="/" />;
  }

  const handleSubmit = (values: ProfileFormValues) => {
    void sessionOps.createProfile(
      values.serverAddress,
      values.name,
      values.password,
      values.persistLogin
    );
  };

  return (
    <LandingPageLayout>
      <ProfileForm
        onSubmit={handleSubmit}
        // error={error?.message}
        secondaryUri="/login"
        secondaryLabel="Log in"
        submitLabel="Create Profile"
      />
    </LandingPageLayout>
  );
};

export default SignUpPage;
