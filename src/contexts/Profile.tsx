import * as React from "react";

import * as pb from "../lib/pb";
import { useLazyCall } from "./Api";

interface State {
  loading: boolean;
  profile: pb.IProfile | null;
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
  profile?: pb.IProfile | null;
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

  const [, createProfile] = useLazyCall("createProfile", { onComplete, onError });
  const [, loadProfile] = useLazyCall("loadProfile", { onComplete, onError });

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

  const handleDone = (profile?: pb.IProfile) =>
    setState((prev) => ({
      ...prev,
      loading: false,
      profile,
    }));

  const [, loadSession] = useLazyCall("loadSession", {
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
