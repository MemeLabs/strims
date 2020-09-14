import qs from "qs";
import { useMemo } from "react";

const useQuery = (queryString: string) =>
  useMemo(() => qs.parse(queryString, { ignoreQueryPrefix: true }) || {}, [queryString]);

export default useQuery;
