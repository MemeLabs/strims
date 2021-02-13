import { useState } from "react";
import { useDebounce } from "react-use";

const useIdleTimout = (timeout = 3000, initialState = true): [boolean, () => void, () => void] => {
  const [idle, setIdle] = useState(initialState);
  const [lastActive, setLastActive] = useState(0);

  useDebounce(() => setIdle(true), timeout, [lastActive]);

  const renewTimeout = () => {
    setIdle(false);
    setLastActive(Date.now());
  };

  const clearTimeout = () => setIdle(true);

  return [idle, renewTimeout, clearTimeout];
};

export default useIdleTimout;
