import { useEffect, useRef } from 'react';

const useMountedFlag = () => {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  return mountedRef;
};

export { useMountedFlag };
