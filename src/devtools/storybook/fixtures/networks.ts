// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

import { Network as DirectoryNetwork } from "../../../apis/strims/network/v1/directory/directory";
import {
  AliasReservation,
  Invitation,
  InvitationV0,
  Network,
  Peer,
  ServerConfig,
} from "../../../apis/strims/network/v1/network";
import { Certificate, CertificateBody, KeyUsage } from "../../../apis/strims/type/certificate";
import { Key, KeyType } from "../../../apis/strims/type/key";
import { bytes, fixtureTime, key } from "./ids";
import { profileKey } from "./profile";

/** Network 1's key, the one the directory and chat mocks' listings name. */
export const testNetworkKey = Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=");

const names = [
  "Test Network",
  "Strims",
  "Comfy Corner",
  "Late Night",
  "Speedrun Club",
  "Film Society",
  "Retro Games",
  "Music Lounge",
];

// Network `i`'s name, from 1.
export const networkName = (i: number): string =>
  i <= names.length ? names[i - 1] : `Network ${i}`;

// Network `i`'s root key, from 1: network 1's is testNetworkKey.
export const networkKey = (i: number): Uint8Array => (i === 1 ? testNetworkKey : key(`network:${i}`));

const encodeBody = (b: ConstructorParameters<typeof CertificateBody>[0]): Uint8Array =>
  CertificateBody.encode(new CertificateBody(b)).finish();

export interface CertificateChainOptions {
  /** The root's key, the network's. */
  networkKey: Uint8Array;
  /** The root's subject, the network's name. */
  networkName: string;
  /** The leaf's key (default the profile's). */
  peerKey?: Uint8Array;
  /** The leaf's subject, the peer's alias in the network. */
  alias?: string;
  /** Validity in seconds since the epoch. */
  notBefore?: bigint;
  notAfter?: bigint;
}

// A network's two certificate chain: a peer certificate (leaf) issued by the
// network's root, whose key and subject name the network
// (lib/certificate's certificateRoot). The signatures are placeholders:
// nothing in the UI checks them.
export const certificateChain = ({
  networkKey,
  networkName,
  peerKey = profileKey,
  alias = "test_user",
  notBefore = fixtureTime(),
  notAfter = fixtureTime(3650),
}: CertificateChainOptions): Certificate => {
  const rootSerial = bytes(`serial:${Base64.fromUint8Array(networkKey)}`, 16);
  return new Certificate({
    body: encodeBody({
      key: peerKey,
      keyType: KeyType.KEY_TYPE_ED25519,
      keyUsage: KeyUsage.KEY_USAGE_PEER | KeyUsage.KEY_USAGE_SIGN,
      subject: alias,
      notBefore,
      notAfter,
      serialNumber: bytes(`serial:${Base64.fromUint8Array(networkKey)}:${alias}`, 16),
      issuerSerialNumber: rootSerial,
    }),
    signature: bytes(`signature:${alias}`, 64),
    parent: new Certificate({
      body: encodeBody({
        key: networkKey,
        keyType: KeyType.KEY_TYPE_ED25519,
        keyUsage: KeyUsage.KEY_USAGE_SIGN,
        subject: networkName,
        notBefore,
        notAfter,
        serialNumber: rootSerial,
      }),
      signature: bytes(`signature:${networkName}`, 64),
    }),
  });
};

export interface NetworkOptions {
  name?: string;
  key?: Uint8Array;
  /** The profile's alias in the network. */
  alias?: string;
  /** Whether the profile runs the network's server (gives it a serverConfig). */
  owner?: boolean;
}

// Network `i` (its id), from 1; network 1 is the profile's own server by default.
export const network = (i: number, opts: NetworkOptions = {}): Network => {
  const { name = networkName(i), key = networkKey(i), alias = "test_user", owner = i === 1 } = opts;
  return new Network({
    id: BigInt(i),
    certificate: certificateChain({ networkKey: key, networkName: name, alias }),
    alias,
    serverConfig: owner
      ? new ServerConfig({
          name,
          key: new Key({
            type: KeyType.KEY_TYPE_ED25519,
            public: key,
            private: bytes(`network:${i}:private`, 64),
          }),
          rootCertTtlSecs: BigInt(10 * 365 * 86400),
          peerCertTtlSecs: BigInt(14 * 86400),
          directory: {
            integrations: {
              angelthump: { enable: true },
              twitch: { enable: false },
              youtube: { enable: true, publicApiKey: "" },
              swarm: { enable: true },
            },
            publishQuota: 10,
            joinQuota: 20,
            broadcastInterval: 15,
            refreshInterval: 300,
            sessionTimeout: 900,
            minPingInterval: 300,
            maxPingInterval: 900,
            embedLoadInterval: 60,
            loadMediaEmbedTimeout: 60,
          },
        })
      : undefined,
  });
};

// Networks 1..count.
export const networks = (count: number): Network[] =>
  Array.from({ length: count }, (_, i) => network(i + 1));

// NetworkContext items for networks 1..count, peer counts 2^i.
export const networkItems = (count: number): { network: Network; peerCount: number }[] =>
  networks(count).map((network, i) => ({ network, peerCount: Math.pow(2, i) }));

// A network as directory messages name it.
export const directoryNetwork = (n: Network): DirectoryNetwork => {
  let root = n.certificate;
  while (root.parent) {
    root = root.parent;
  }
  const body = CertificateBody.decode(root.body);
  return new DirectoryNetwork({ id: n.id, key: body.key, name: body.subject });
};

const peerAliases = ["jbpratt", "soadsoap", "widdlez", "KartoffelKopf", "tahley", "biscophan"];

// `count` peers of network `networkId`: the first an admin, the fourth
// banned, ids networkId * 1000 + i.
export const peers = (networkId: bigint, count = peerAliases.length): Peer[] =>
  Array.from(
    { length: count },
    (_, i) =>
      new Peer({
        id: networkId * BigInt(1000) + BigInt(i + 1),
        networkId,
        publicKey: key(`peer:${networkId}:${i}`),
        inviterPeerId: i === 0 ? BigInt(0) : networkId * BigInt(1000) + BigInt(1),
        inviteQuota: i === 0 ? 100 : 5,
        createdAt: fixtureTime(i * 7),
        isAdmin: i === 0,
        isBanned: i === 3,
        alias: peerAliases[i % peerAliases.length] + (i < peerAliases.length ? "" : `${i}`),
        aliasChangedAt: fixtureTime(i * 7 + 1),
      })
  );

// `count` alias reservations in network `networkId`.
export const aliasReservations = (networkId: bigint, count = 3): AliasReservation[] =>
  Array.from(
    { length: count },
    (_, i) =>
      new AliasReservation({
        id: networkId * BigInt(1000) + BigInt(i + 1),
        networkId,
        alias: `reserved_${i + 1}`,
        peerKey: key(`reservation:${networkId}:${i}`),
        reservedUntil: fixtureTime(30 + i * 30),
      })
  );

// An invitation to `n`, encoded as CreateInvitation answers it.
export const invitation = (n: Network): Invitation =>
  new Invitation({
    version: 0,
    data: InvitationV0.encode(
      new InvitationV0({
        key: new Key({
          type: KeyType.KEY_TYPE_ED25519,
          public: key(`invitation:${n.id}`),
          private: bytes(`invitation:${n.id}:private`, 64),
        }),
        certificate: n.certificate,
        networkName: directoryNetwork(n).name,
        bootstrapClients: [
          { id: BigInt(1), clientOptions: { websocketOptions: { url: "wss://strims.gg/_bootstrap" } } },
        ],
      })
    ).finish(),
  });
