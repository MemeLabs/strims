import { Certificate } from "../apis/strims/type/certificate";

export const certificateRoot = (cert: Certificate): Certificate =>
  cert.parentOneof.case === Certificate.ParentOneofCase.PARENT
    ? certificateRoot(cert.parentOneof.parent)
    : cert;
