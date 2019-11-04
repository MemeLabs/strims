

import {useMemo} from 'react';
import qs from 'qs';

const useQuery = queryString => useMemo(() => {
  return qs.parse(queryString, {ignoreQueryPrefix: true}) || {};
}, [queryString]);

export default useQuery;
