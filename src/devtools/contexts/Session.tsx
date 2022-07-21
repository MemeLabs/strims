// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useMemo } from "react";

import { ISignInRequest, ISignUpRequest } from "../../apis/strims/auth/v1/auth";
import { Profile } from "../../apis/strims/profile/v1/profile";
import { Key } from "../../apis/strims/type/key";
import { Ops, SessionContext, State } from "../../contexts/Session";

export interface SessionProviderProps {
  children: ReactNode;
}

const state: State = {
  linkedProfiles: [],
  profile: new Profile({
    key: new Key(),
  }),
  loading: false,
};

const createProfile = (serverAddress: string, req: ISignUpRequest) => Promise.resolve();
const signIn = (serverAddress: string, req: ISignInRequest) => Promise.resolve();

export const SessionProvider: React.FC<SessionProviderProps> = ({ children }) => {
  const value = useMemo<[State, Ops]>(() => [state, { createProfile, signIn }], []);
  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
};
