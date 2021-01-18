import qs from "qs";
import { useMemo } from "react";

const useQuery = <T>(queryString: string): T =>
  (useMemo(() => qs.parse(queryString, { ignoreQueryPrefix: true }) || {}, [
    queryString,
  ]) as unknown) as T;

export default useQuery;
