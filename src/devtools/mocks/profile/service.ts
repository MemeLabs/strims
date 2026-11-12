// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as profilev1 from "../../../apis/strims/profile/v1/profile";
import { Device, Profile } from "../../../apis/strims/profile/v1/profile";
import {
  ProfileFrontendService,
  UnimplementedProfileFrontendService,
} from "../../../apis/strims/profile/v1/profile_rpc";
import { fixtureTime } from "../../storybook/fixtures/ids";
import { profile as fixtureProfile } from "../../storybook/fixtures/profile";

const kinds: [os: string, device: string][] = [
  ["web", "Firefox"],
  ["linux", ""],
  ["web", "Chrome"],
  ["windows", ""],
  ["web", "Safari"],
];

// Device ids are replica ids: large and random-looking.
const deviceId = (i: number): bigint => BigInt("0x3f1a9c27d4e8b06") * BigInt(i + 1) + BigInt(i);

// `count` devices, the first the profile's current one (fixture profile's
// deviceId is its id), the rest logged in on earlier days.
export const devices = (count = 4): Device[] =>
  Array.from({ length: count }, (_, i) => {
    const [os, device] = kinds[i % kinds.length];
    return new Device({
      id: i === 0 ? BigInt(1) : deviceId(i),
      os,
      device,
      lastLogin: fixtureTime(300 - i * 37),
    });
  });

export interface ProfileServiceOptions {
  profile?: Profile;
  /** The profile's devices, the current one among them; default devices(4). */
  devices?: Device[];
  /** The device this session runs on; default the profile's deviceId. */
  currentDeviceId?: bigint;
}

const errNotFound = (what: string) => new Error(`${what} not found`);

// The signed-in profile and its devices in memory. listDevices answers the
// current device apart from the others, as the node does.
export default class ProfileService
  extends UnimplementedProfileFrontendService
  implements ProfileFrontendService
{
  profile: Profile;
  devices: Device[];
  currentDeviceId: bigint;

  constructor({ profile, devices: d, currentDeviceId }: ProfileServiceOptions = {}) {
    super();
    this.profile = profile ?? fixtureProfile();
    this.devices = d ?? devices();
    this.currentDeviceId = currentDeviceId ?? this.profile.deviceId;
  }

  get(): profilev1.GetProfileResponse {
    return new profilev1.GetProfileResponse({ profile: this.profile });
  }

  update(req: profilev1.UpdateProfileRequest): profilev1.UpdateProfileResponse {
    this.profile = new Profile({ ...this.profile, name: req.name || this.profile.name });
    return new profilev1.UpdateProfileResponse({ profile: this.profile });
  }

  getDevice(req: profilev1.GetDeviceRequest): profilev1.GetDeviceResponse {
    const device = this.devices.find((d) => d.id === req.id);
    if (!device) {
      throw errNotFound(`device ${req.id}`);
    }
    return new profilev1.GetDeviceResponse({ device });
  }

  listDevices(): profilev1.ListDevicesResponse {
    return new profilev1.ListDevicesResponse({
      devices: this.devices.filter((d) => d.id !== this.currentDeviceId),
      currentDevice: this.devices.find((d) => d.id === this.currentDeviceId),
    });
  }

  deleteDevice(req: profilev1.DeleteDeviceRequest): profilev1.DeleteDeviceResponse {
    if (req.id === this.currentDeviceId) {
      throw new Error("the current device can't be deleted");
    }
    this.getDevice(new profilev1.GetDeviceRequest({ id: req.id }));
    this.devices = this.devices.filter((d) => d.id !== req.id);
    return new profilev1.DeleteDeviceResponse();
  }
}
