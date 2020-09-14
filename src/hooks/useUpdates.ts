import { useEffect, useRef } from "react";

const useUpdates = (effect: () => void, deps: [any]) => {
  const init = useRef(false);

  useEffect(() => {
    if (init.current) {
      effect();
    }
  }, deps);

  useEffect(() => {
    init.current = true;
  }, []);
};

export default useUpdates;
