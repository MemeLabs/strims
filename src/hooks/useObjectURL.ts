import { useEffect, useMemo } from "react";

const useObjectURL = (type: string, data: Uint8Array): string => {
  const url = useMemo(() => URL.createObjectURL(new Blob([data], { type })), [type, data]);
  useEffect(() => () => URL.revokeObjectURL(url), [url]);
  return url;
};

export default useObjectURL;
