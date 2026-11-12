// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Landing.stories.scss";

import { Base64 } from "js-base64";
import React, { ReactNode, useEffect, useRef } from "react";
import { useForm } from "react-hook-form";

import { PairingToken } from "../../../../apis/strims/auth/v1/auth";
import PairingTokenInput from "../../../../components/Landing/PairingTokenInput";
import { ProfileFormValues } from "../../../../components/Landing/ProfileForm";
import LandingPageLayout from "../../../../components/LandingPageLayout";
import { Story, fixtures } from "../../../storybook";

// A pairing token for the fixture profile (test_user), as a device shares it.
const validToken = () =>
  Base64.fromUint8Array(
    PairingToken.encode(new PairingToken({ profile: fixtures.profile() })).finish()
  );

// The input in a profile form on the landing page, as ProfileForm has it.
// Scanning needs the camera, so no story opens the scanner.
const Form: React.FC<{ token?: string; children?: ReactNode }> = ({ token = "", children }) => {
  const { control } = useForm<ProfileFormValues>({
    defaultValues: { pairingTokenString: token },
  });
  return (
    <div className="landing_story">
      <LandingPageLayout>
        <form onSubmit={(e) => e.preventDefault()}>
          <PairingTokenInput control={control} />
          {children}
        </form>
      </LandingPageLayout>
    </div>
  );
};

// Clicks Paste once mounted, which opens the paste modal.
const PasteOpened: React.FC = () => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const paste = [...ref.current.querySelectorAll("button")].find(
      (b) => b.textContent === "Paste"
    );
    paste.click();
  }, []);
  return (
    <div ref={ref} style={{ height: "100%" }}>
      <Form />
    </div>
  );
};

const stories: Story[] = [
  {
    name: "Empty",
    component: () => <Form />,
  },
  {
    name: "Valid token",
    component: () => <Form token={validToken()} />,
  },
  {
    name: "Invalid token",
    component: () => <Form token="bm90IGEgcGFpcmluZyB0b2tlbg" />,
  },
  {
    name: "Paste modal",
    component: () => <PasteOpened />,
  },
];

export default stories;
