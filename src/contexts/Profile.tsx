import * as React from "react";

import { Profile } from "../apis/strims/profile/v1/profile";
import { useLazyCall } from "./FrontendApi";

interface State {
  loading: boolean;
  profile: Profile | null;
  error: Error | null;
}

const initialState: State = {
  loading: true,
  profile: null,
  error: null,
};

const ProfileContext = React.createContext<[State, React.Dispatch<React.SetStateAction<State>>]>(
  null
);

interface LoginResponse {
  profile?: Profile | null;
  sessionId?: string | null;
}

export const useProfile = () => {
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
  return [state, actions] as [State, typeof actions];
};

export const Provider = ({ children }: any) => {
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
      loadSession({ sessionId });
    } else {
      handleDone();
    }
  }, []);

  return <ProfileContext.Provider value={[state, setState]}>{children}</ProfileContext.Provider>;
};

Provider.displayName = "Profile.Provider";
