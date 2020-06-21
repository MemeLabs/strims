import qs from "qs";
import { useMemo } from "react";

const useQuery = (queryString) =>
  useMemo(() => {
    return qs.parse(queryString, { ignoreQueryPrefix: true }) || {};
  }, [queryString]);

export default useQuery;
