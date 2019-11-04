import {useEffect, useState} from 'react';

const useUpdates = (effect, deps) => {
  const [init] = useState({});

  useEffect(() => {
    if (init.done) {
      effect();
    }
  }, deps);

  useEffect(() => {
    init.done = true;
  }, []);
};

export default useUpdates;
