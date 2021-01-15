import { Certificate } from "../apis/strims/type/certificate";

export const rootCertificate = (cert: Certificate): Certificate =>
  cert.parentOneof.case === Certificate.ParentOneofCase.PARENT
    ? rootCertificate(cert.parentOneof.parent)
    : cert;
