// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useLocation } from "react-router-dom";

import useQuery from "../hooks/useQuery";

const VALID_NEXT_PATH = /^\/\w[\w/_\-.?=#%&]*$/;

interface NextQueryParams {
  next: string;
}

const useNextQuery = () => {
  const { search } = useLocation();
  const { next } = useQuery<NextQueryParams>(search);

  return VALID_NEXT_PATH.test(next) ? next : undefined;
};

export default useNextQuery;
