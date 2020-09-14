import { useEffect } from "react";

const useReady = (effect, deps) =>
  useEffect(() => {
    for (let i = 0; i < deps.length; i++) {
      if (deps[i] == null) {
        return;
      }
    }

    return effect();
  }, deps);

export default useReady;
