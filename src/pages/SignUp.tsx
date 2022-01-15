import React, { useEffect } from "react";
import { Navigate } from "react-router-dom";

import ProfileForm, { ProfileFormValues } from "../components/Landing/ProfileForm";
import LandingPageLayout from "../components/LandingPageLayout";
import { useCall } from "../contexts/FrontendApi";
import { useClient } from "../contexts/FrontendApi";
import { useProfile } from "../contexts/Profile";
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
    // console.log(values);
    // // void profileActions.createProfile(data)
    // const res = client.auth.signUp({
    //   name: values.name,
    //   password: values.password,
    //   persistLogin: values.persistLogin,
    // });
    // res.then(
    //   (data) => console.log(data),
    //   (err) => console.log(err)
    // );
  };

  useEffect(() => {
    console.log(session);
  }, [session]);

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
