// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only


/**
 * The network's error codes, 1-999: the CA's refusals, a peer's, the
 * bootstrap's and the network frontend's. A network record's
 * certificate_renewal_error holds the code of its last renewal's failure,
 * UNDEFINED when it renewed, whichever API's code that is.
 */
export enum ErrorCode {
  /** No code, or no error. */
  UNDEFINED = 0,
  /** A failure the sender gave no code. */
  UNKNOWN = 1,
  ALIAS_IN_USE = 2,
  ALIAS_CHANGE_COOLDOWN_VIOLATIED = 3,
  INVITATION_QUOTA_EXCEEDED = 4,
  INVITER_BANNED = 5,
  /**
   * The CA can't use the request: a field is missing, malformed or over
   * its bound.
   */
  INVALID_CA_REQUEST = 6,
  /** The network has no server config, so this side isn't its CA. */
  NOT_CA = 7,
  /** The CA banned the peer. */
  PEER_BANNED = 8,
  /** The CA issued no certificate that matches the query. */
  CERTIFICATE_NOT_FOUND = 9,
  /** The peer renews more often than the CA allows. */
  TOO_MANY_RENEWALS = 10,
  /** The network's CA isn't published in its hash table. */
  CA_NOT_PUBLISHED = 11,
  /** The CA's membership isn't trusted yet. */
  MEMBERSHIP_NOT_TRUSTED = 12,
  /** The CA's answer holds no certificate, or one that isn't the caller's. */
  INVALID_CERTIFICATE_ISSUED = 13,
  /** The peer has no link to this node. */
  NOT_LINKED = 14,
  /** A request lacks its certificate. */
  MISSING_CERTIFICATE = 15,
  /** A certificate doesn't decode or verify. */
  INVALID_CERTIFICATE = 16,
  /** A certificate's key isn't the peer's. */
  OWNER_MISMATCH = 17,
  /** A certificate is provisional. */
  PROVISIONAL_CERTIFICATE = 18,
  /** No binding of the network with the peer. */
  BINDING_NOT_FOUND = 19,
  /** The network control has stopped. */
  CONTROL_STOPPED = 20,
  /** The profile has no key. */
  NO_PROFILE_KEY = 21,
  /**
   * The network has no certificate of the profile's, or the node none for
   * its host.
   */
  NO_CERTIFICATE = 22,
  /**
   * An invitation is incomplete, malformed, of an unsupported version or
   * over its bound.
   */
  INVALID_INVITATION = 23,
  /** A signature or key operation failed. */
  CRYPTO = 24,
  /** The node doesn't run the network, or the profile has no such network. */
  NETWORK_NOT_FOUND = 25,
  /** The node hasn't joined the network. */
  NOT_JOINED = 26,
  /** The node has no link to the peer. */
  PEER_NOT_FOUND = 27,
  /** The profile has no such alias reservation. */
  ALIAS_RESERVATION_NOT_FOUND = 28,
  /** A reservation's alias is in use, so its cooldown stays. */
  ALIAS_RESERVATION_IN_USE = 29,
  /** An alias is over its bound. */
  ALIAS_TOO_LONG = 30,
  /** A network record lacks its server config. */
  SERVER_CONFIG_MISSING = 31,
  /** A network's key can't change. */
  NETWORK_KEY_CHANGED = 32,
  /** The profile has no such bootstrap client. */
  BOOTSTRAP_CLIENT_NOT_FOUND = 33,
  /** A bootstrap client's url isn't ws:// or wss://, or is over its bound. */
  INVALID_BOOTSTRAP_URL = 34,
  /** A request lacks its config. */
  CONFIG_MISSING = 35,
  /** A bootstrap client's options are of a type the node doesn't run. */
  INVALID_CLIENT_OPTIONS = 36,
  /** The profile has as many bootstrap clients as it may. */
  TOO_MANY_BOOTSTRAP_CLIENTS = 37,
  /** The peer, or this node, doesn't bootstrap networks. */
  BOOTSTRAP_UNSUPPORTED = 38,
  /** Only a network this profile manages can be published. */
  NETWORK_NOT_MANAGED = 39,
  /** A network record has no key. */
  NETWORK_HAS_NO_KEY = 40,
  /** A certificate's chain is over its bound. */
  CERTIFICATE_CHAIN_TOO_LONG = 41,
  /** A certificate isn't the profile's. */
  CERTIFICATE_NOT_FOR_PROFILE = 42,
  /** A certificate isn't signed by its network. */
  CERTIFICATE_NOT_SIGNED_BY_NETWORK = 43,
  /** The profile has the network already. */
  NETWORK_EXISTS = 44,
  /** The profile has as many networks as it may. */
  TOO_MANY_NETWORKS = 45,
}

