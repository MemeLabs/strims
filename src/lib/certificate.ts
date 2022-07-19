// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Certificate } from "../apis/strims/type/certificate";

export const certificateRoot = (cert: Certificate): Certificate =>
  cert.parentOneof.case === Certificate.ParentOneofCase.PARENT
    ? certificateRoot(cert.parentOneof.parent)
    : cert;

export const certificateChain = (cert: Certificate) => {
  const chain: Certificate[] = [];
  while (cert) {
    chain.unshift(cert);
    cert =
      cert.parentOneof.case === Certificate.ParentOneofCase.PARENT ? cert.parentOneof.parent : null;
  }
  return chain;
};
