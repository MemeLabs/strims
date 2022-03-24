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
    void sessionOps.createProfile(values.serverAddress, {
      name: values.name,
      password: values.password,
      persistLogin: values.persistLogin,
    });
  };

  return (
    <LandingPageLayout>
      <ProfileForm
        onSubmit={handleSubmit}
        // error={error?.message}
        secondaryUri="/login"
        secondaryLabel="Log in"
        submitLabel="Create Profile"
        defaultValues={{
          persistLogin: true,
        }}
      />
    </LandingPageLayout>
  );
};

export default SignUpPage;
