import React from "react";

import { FrontendClient } from "../apis/client";
import { Profile } from "../apis/strims/profile/v1/profile";
import { CallHookDispatcher } from "./Api";
import { useLazyCall } from "./FrontendApi";

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
  createProfile: CallHookDispatcher<FrontendClient, "profile", "create">;
  loadProfile: CallHookDispatcher<FrontendClient, "profile", "load">;
  clearProfile: () => void;
  clearError: () => void;
};

const ProfileContext =
  React.createContext<[UseProfileState, React.Dispatch<React.SetStateAction<UseProfileState>>]>(
    null
  );

interface LoginResponse {
  profile?: Profile | null;
  sessionId?: string | null;
}

export const useProfile = (): [UseProfileState, UseProfileActions] => {
  const [state, setState] = React.useContext(ProfileContext);

  const onComplete = ({ profile, sessionId }: LoginResponse) => {
    sessionStorage.setItem("sessionId", sessionId);
    setState((prev) => ({
      ...prev,
      loading: false,
      error: null,
      profile,
    }));
  };

  const onError = (error: Error) =>
    setState((prev) => ({
      ...prev,
      loading: false,
      profile: null,
      error,
    }));

  const [, createProfile] = useLazyCall("profile", "create", { onComplete, onError });
  const [, loadProfile] = useLazyCall("profile", "load", { onComplete, onError });

  const clearProfile = () => {
    sessionStorage.removeItem("sessionId");
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
    createProfile,
    loadProfile,
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

  const [, loadSession] = useLazyCall("profile", "loadSession", {
    onComplete: ({ profile }) => handleDone(profile),
    onError: () => handleDone(),
  });

  React.useEffect(() => {
    const sessionId = sessionStorage.getItem("sessionId");
    if (sessionId) {
      void loadSession({ sessionId });
    } else {
      handleDone();
    }
  }, []);

  return <ProfileContext.Provider value={[state, setState]}>{children}</ProfileContext.Provider>;
};

Provider.displayName = "Profile.Provider";
