// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { Profile } from "../apis/strims/profile/v1/profile";
import { useCall } from "./FrontendApi";

export interface UseProfileState {
  loading: boolean;
  profile: Profile | null;
  error: Error | null;
}

const initialState: UseProfileState = {
  loading: true,
  profile: null,
  error: null,
};

export type UseProfileActions = {
  clearProfile: () => void;
  clearError: () => void;
};

const ProfileContext =
  React.createContext<[UseProfileState, React.Dispatch<React.SetStateAction<UseProfileState>>]>(
    null
  );

export const useProfile = (): [UseProfileState, UseProfileActions] => {
  const [state, setState] = React.useContext(ProfileContext);

  const clearProfile = () => {
    setState((prev) => ({
      ...prev,
      profile: null,
    }));
  };

  const clearError = () =>
    setState((prev) => ({
      ...prev,
      error: null,
    }));

  const actions = {
    clearProfile,
    clearError,
  };
  return [state, actions];
};

export const Provider: React.FC = ({ children }) => {
  const [state, setState] = React.useState(initialState);

  const handleDone = (profile?: Profile) =>
    setState((prev) => ({
      ...prev,
      loading: false,
      profile,
    }));

  useCall("profile", "get", {
    onComplete: ({ profile }) => handleDone(profile),
    onError: () => handleDone(),
  });

  return <ProfileContext.Provider value={[state, setState]}>{children}</ProfileContext.Provider>;
};

Provider.displayName = "Profile.Provider";
