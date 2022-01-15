import { Readable, Writable } from "stream";

import { DBSchema, IDBPDatabase, openDB } from "idb";
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

import { FrontendClient } from "../apis/client";
import { ISignInRequest, LinkedProfile } from "../apis/strims/auth/v1/auth";
import { Profile } from "../apis/strims/profile/v1/profile";
import { Provider as ApiProvider } from "../contexts/FrontendApi";

export interface ClientConstructor<T> {
  new (w: Writable, r: Readable): T;
}

export interface Conn {
  client<T>(C: ClientConstructor<T>): Promise<T>;
  close(): void;
}

export interface ConnFactoryThing {
  local(): Conn;
  remote(address: string): Conn;
}

interface State {
  linkedProfiles: LinkedProfile[];
  profile: Profile;
  loading: boolean;
}

type Ops = {
  createProfile: (
    serverAddress: string,
    name: string,
    password: string,
    persistLogin: boolean
  ) => Promise<void>;
  signIn: (serverAddress: string, req: ISignInRequest) => Promise<void>;
};

const initialState: State = {
  linkedProfiles: [],
  profile: null,
  loading: false,
};

export const SessionContext = createContext<[State, Ops]>(null);

interface ProviderProps {
  thing: ConnFactoryThing;
}

interface LinkedProfilesDBSchema extends DBSchema {
  "data": {
    key: string;
    value: Uint8Array;
  };
  "sequence": {
    key: string;
    value: string;
  };
}

class LinkedProfilesDB {
  private db: Promise<IDBPDatabase<LinkedProfilesDBSchema>>;

  constructor() {
    this.db = openDB<LinkedProfilesDBSchema>("linkedProfiles", 1, {
      upgrade: (db) => {
        db.createObjectStore("data");
        db.createObjectStore("sequence");
      },
    });
  }

  async nextId(): Promise<bigint> {
    const db = await this.db;
    const tx = db.transaction("sequence", "readwrite");
    const value = BigInt((await tx.objectStore("sequence").get("id")) || "1");
    await tx.objectStore("sequence").put((value + BigInt(1)).toString(), "id");
    tx.commit();
    return value;
  }

  async getLogins(): Promise<LinkedProfile[]> {
    const db = await this.db;
    const bs = await db.getAll("data");
    return bs.map((b) => LinkedProfile.decode(b));
  }

  async insertProfile(profile: LinkedProfile): Promise<void> {
    const db = await this.db;
    await db.put("data", LinkedProfile.encode(profile).finish().slice(), profile.id.toString());
  }
}

export const Provider: React.FC<ProviderProps> = ({ thing, children }) => {
  const [state, setState] = useState<State>(initialState);
  const [db] = useState(() => new LinkedProfilesDB());
  const [conn, setConn] = useState<{ conn: Conn; client: FrontendClient }>(null);
  useEffect(() => () => conn?.conn.close(), [conn]);

  // TODO: init state from... default? current? profile

  const insertProfiles = useCallback((...profiles: LinkedProfile[]) => {
    setState((prev) => ({
      ...prev,
      linkedProfiles: [
        ...prev.linkedProfiles.filter((a) => !profiles.some((b) => a.id === b.id)),
        ...profiles,
      ],
    }));
  }, []);

  useEffect(() => {
    void db.getLogins().then((profiles) => insertProfiles(...profiles));
  }, []);

  const createProfile = useCallback(
    async (serverAddress: string, name: string, password: string, persistLogin: boolean) => {
      const conn = serverAddress ? thing.remote(serverAddress) : thing.local();
      const client = await conn.client(FrontendClient);
      const res = await client.auth.signUp({ name, password, persistLogin });

      const profile = new LinkedProfile({
        ...res.linkedProfile,
        id: await db.nextId(),
        serverAddress,
      });
      await db.insertProfile(profile);
      insertProfiles(profile);

      setState((prev) => ({ ...prev, profile: res.profile }));
      setConn({ conn, client });
    },
    []
  );

  const signIn = useCallback(async (serverAddress: string, req: ISignInRequest) => {
    const conn = serverAddress ? thing.remote(serverAddress) : thing.local();
    const client = await conn.client(FrontendClient);
    const res = await client.auth.signIn(req);

    const prev = (await db.getLogins()).find(
      (p) => p.name === res.linkedProfile.name && p.serverAddress === serverAddress
    );

    if (!prev || res.linkedProfile.credentials.case !== LinkedProfile.CredentialsCase.NOT_SET) {
      const profile = new LinkedProfile({
        ...res.linkedProfile,
        id: prev ? prev.id : await db.nextId(),
        serverAddress,
      });
      await db.insertProfile(profile);
      insertProfiles(profile);
    }

    setState((prev) => ({ ...prev, profile: res.profile }));
    setConn({ conn, client });
  }, []);

  const value = useMemo<[State, Ops]>(() => [state, { createProfile, signIn }], [state]);

  return (
    <SessionContext.Provider value={value}>
      <ApiProvider value={conn?.client}>{children}</ApiProvider>
    </SessionContext.Provider>
  );
};

Provider.displayName = "Session.Provider";

export const useSession = () => useContext(SessionContext);
