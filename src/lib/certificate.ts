// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Certificate, CertificateBody } from "../apis/strims/type/certificate";

// A certificate's signed body: its key, subject, validity and serial number.
export const certificateBody = (cert: Certificate): CertificateBody =>
  CertificateBody.decode(cert.body);

// The body of the last certificate in the chain, which names the network.
export const certificateRoot = (cert: Certificate): CertificateBody => {
  while (cert.parent) {
    cert = cert.parent;
  }
  return certificateBody(cert);
};

// The bodies of the chain's certificates, root first.
export const certificateChain = (cert: Certificate): CertificateBody[] => {
  const chain: CertificateBody[] = [];
  while (cert) {
    chain.unshift(certificateBody(cert));
    cert = cert.parent;
  }
  return chain;
};
