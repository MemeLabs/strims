// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

import { Profile } from "../../../apis/strims/profile/v1/profile";
import { Key, KeyType } from "../../../apis/strims/type/key";
import { bytes } from "./ids";

/** The profile's public key: the key its network certificates carry. */
export const profileKey = Base64.toUint8Array("1cFuxhf5HI5d6XQgypXXfqI8LducWmeJtHpUIADR1aU=");

// The signed-in profile.
export const profile = (): Profile =>
  new Profile({
    id: BigInt(1),
    name: "test_user",
    secret: bytes("profile:secret", 32),
    key: new Key({
      type: KeyType.KEY_TYPE_ED25519,
      private: bytes("profile:private", 64),
      public: profileKey,
    }),
    deviceId: BigInt(1),
  });
