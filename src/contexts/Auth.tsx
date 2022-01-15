import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

import { ClientUserThing } from "../apis/strims/auth/v1/auth";
import { useClient } from "./FrontendApi";

interface State {
  things: ClientUserThing[];
}

type Ops = {
  updateDisplayOrder: (srcIndex: number, dstIndex: number) => void;
};

const initialState: State = {
  things: [],
};

export const AuthContext = createContext<[State, Ops]>(null);

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [items, setItems] = useState<State>(initialState);

  // const addItem = (item: Auth) =>
  //   setItems((prev) => {
  //     const next = Array.from(prev);
  //     const i = next.findIndex(
  //       ({ network: { displayOrder } }) => displayOrder > item.network.displayOrder
  //     );
  //     next.splice(i === -1 ? next.length : i, 0, item);
  //     return next;
  //   });

  // const removeItem = (networkId: bigint) =>
  //   setItems((prev) => prev.filter((item) => item.network.id !== networkId));

  const updateDisplayOrder = useCallback(
    (srcIndex: number, dstIndex: number) =>
      setItems((prev) => {
        // const next = Array.from(prev);
        // const [target] = next.splice(srcIndex, 1);
        // next.splice(dstIndex, 0, target);
        // void client.network.updateDisplayOrder({
        //   networkIds: next.map(({ network: { id } }) => id),
        // });
        // return next;
        return prev;
      }),
    []
  );

  useEffect(() => {
    const req = indexedDB.open("auth");
    req.onupgradeneeded = () => {
      req.result.createObjectStore("users");
    };
  }, []);

  const value = useMemo<[State, Ops]>(
    () => [items, { updateDisplayOrder }],
    [items, updateDisplayOrder]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
};

Provider.displayName = "Auth.Provider";

export const useAuth = () => useContext(AuthContext);
