// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import qs from "qs";
import React from "react";
import { useTranslation } from "react-i18next";
import { Navigate } from "react-router-dom";
import { useTitle } from "react-use";

import ProfileForm, { ProfileFormValues } from "../components/Landing/ProfileForm";
import LandingPageLayout from "../components/LandingPageLayout";
import LoadingPlaceholder from "../components/LoadingPlaceholder";
import { useSession } from "../contexts/Session";
import useNextQuery from "../hooks/useNextQuery";

const SignUpPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("landing.signUp.title"));

  const [session, { createProfile }] = useSession();
  const next = useNextQuery();

  if (session.loading) {
    return <LoadingPlaceholder />;
  }
  if (session.profile) {
    return <Navigate to={next ?? "/"} />;
  }

  const handleSubmit = (values: ProfileFormValues) => {
    void createProfile(values.serverAddress, {
      name: values.name,
      password: values.password,
      persistLogin: values.persistLogin,
    });
  };

  return (
    <LandingPageLayout>
      <ProfileForm
        onSubmit={handleSubmit}
        error={session.error?.message}
        secondaryUri={`/login${qs.stringify({ next }, { addQueryPrefix: true })}`}
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
