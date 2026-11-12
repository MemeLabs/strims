// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Readable } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React, { ReactNode, createContext, useContext, useEffect, useState } from "react";

import { FrontendClient } from "../../apis/client";
import { answeredRejection } from "./host/recorder";
import {
  AuthFrontendService,
  registerAuthFrontendService,
} from "../../apis/strims/auth/v1/auth_rpc";
import {
  AutoseedFrontendService,
  registerAutoseedFrontendService,
} from "../../apis/strims/autoseed/v1/autoseed_rpc";
import {
  ChatFrontendService,
  ChatServerFrontendService,
  registerChatFrontendService,
  registerChatServerFrontendService,
} from "../../apis/strims/chat/v1/chat_rpc";
import { DebugService, registerDebugService } from "../../apis/strims/debug/v1/debug_rpc";
import {
  BootstrapFrontendService,
  registerBootstrapFrontendService,
} from "../../apis/strims/network/v1/bootstrap/bootstrap_rpc";
import {
  DirectoryFrontendService,
  registerDirectoryFrontendService,
} from "../../apis/strims/network/v1/directory/directory_rpc";
import {
  NetworkFrontendService,
  registerNetworkFrontendService,
} from "../../apis/strims/network/v1/network_rpc";
import {
  NotificationFrontendService,
  registerNotificationFrontendService,
} from "../../apis/strims/notification/v1/notification_rpc";
import {
  ProfileFrontendService,
  registerProfileFrontendService,
} from "../../apis/strims/profile/v1/profile_rpc";
import {
  ReplicationFrontendService,
  registerReplicationFrontendService,
} from "../../apis/strims/replication/v1/replication_rpc";
import { CaptureService, registerCaptureService } from "../../apis/strims/video/v1/capture_rpc";
import {
  VideoChannelFrontendService,
  registerVideoChannelFrontendService,
} from "../../apis/strims/video/v1/channel_rpc";
import { EgressService, registerEgressService } from "../../apis/strims/video/v1/egress_rpc";
import {
  HLSEgressService,
  registerHLSEgressService,
} from "../../apis/strims/video/v1/hls_egress_rpc";
import {
  VideoIngressService,
  registerVideoIngressService,
} from "../../apis/strims/video/v1/ingress_rpc";
import { VNICFrontendService, registerVNICFrontendService } from "../../apis/strims/vnic/v1/vnic_rpc";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { AsyncPassThrough } from "../../lib/stream";

/** The services a FrontendClient's fields call, by field name. */
export interface ServiceTypes {
  auth: AuthFrontendService;
  autoseed: AutoseedFrontendService;
  bootstrap: BootstrapFrontendService;
  chat: ChatFrontendService;
  chatServer: ChatServerFrontendService;
  debug: DebugService;
  directory: DirectoryFrontendService;
  network: NetworkFrontendService;
  notification: NotificationFrontendService;
  profile: ProfileFrontendService;
  replication: ReplicationFrontendService;
  videoCapture: CaptureService;
  videoChannel: VideoChannelFrontendService;
  videoEgress: EgressService;
  videoIngress: VideoIngressService;
  hlsEgress: HLSEgressService;
  vnic: VNICFrontendService;
}

/**
 * Mock services by FrontendClient field. A service may implement only some
 * methods: a call to another logs a console error (failing the story) and
 * answers an RPC error naming it.
 */
export type MockServices = { [K in keyof ServiceTypes]?: Partial<ServiceTypes[K]> };

type Registrar<T> = (registry: ServiceRegistry, service: T) => void;

const registrars: { [K in keyof ServiceTypes]: Registrar<ServiceTypes[K]> } = {
  auth: registerAuthFrontendService,
  autoseed: registerAutoseedFrontendService,
  bootstrap: registerBootstrapFrontendService,
  chat: registerChatFrontendService,
  chatServer: registerChatServerFrontendService,
  debug: registerDebugService,
  directory: registerDirectoryFrontendService,
  network: registerNetworkFrontendService,
  notification: registerNotificationFrontendService,
  profile: registerProfileFrontendService,
  replication: registerReplicationFrontendService,
  videoCapture: registerCaptureService,
  videoChannel: registerVideoChannelFrontendService,
  videoEgress: registerEgressService,
  videoIngress: registerVideoIngressService,
  hlsEgress: registerHLSEgressService,
  vnic: registerVNICFrontendService,
};

type Method = (...args: unknown[]) => unknown;

// A mock's mistake is the story's error too: logged, so the storybook
// records it even where the component handles the RPC error its call gets.
const mockError = (message: string): Error => {
  // eslint-disable-next-line no-console
  console.error(message);
  return new Error(message);
};

// The RPC host encodes whatever a method returns with its constructor's
// encode, so a plain object fails inside it (and the caller times out after
// 5 s), and the caller never hears of undefined: name the mistake instead.
const checkMessage = (name: string, value: unknown): void => {
  const encode = (value?.constructor as { encode?: unknown })?.encode;
  if (typeof encode !== "function") {
    throw mockError(
      `storybook: ${name} gave ${value === undefined ? "undefined" : "a plain object"}; ` +
        "return a message (new XResponse(...))"
    );
  }
};

const checked = (name: string, method: Method, self: object): Method => {
  const call = (...args: unknown[]): unknown => {
    let res: unknown;
    try {
      res = Reflect.apply(method, self, args);
    } catch (e) {
      // The generated Unimplemented* services' methods throw this.
      throw (e as Error)?.message === "not implemented"
        ? mockError(`storybook: ${name} is not mocked`)
        : e;
    }
    if (res instanceof Promise) {
      return res.then(
        (v) => {
          checkMessage(name, v);
          return v as unknown;
        },
        (e: unknown) => {
          answeredRejection(e);
          throw e;
        }
      );
    }
    if (res instanceof Readable) {
      res.on("data", (v) => checkMessage(`${name}'s stream`, v));
      return res;
    }
    checkMessage(name, res);
    return res;
  };
  return call;
};

// Methods the service lacks (or an Unimplemented* base's) log and throw a
// named error, which the host answers as an RPC error.
const complete = <T extends object>(field: string, service: T): T =>
  new Proxy(service, {
    get: (target, prop, receiver) => {
      const name = `${field}.${String(prop)}`;
      const value: unknown = Reflect.get(target, prop, receiver);
      if (typeof value !== "function") {
        return () => {
          throw mockError(`storybook: ${name} is not mocked`);
        };
      }
      return checked(name, value as Method, target);
    },
  });

// A FrontendClient whose calls the given mock services answer, in the page.
// Every field's service is registered, those not given as empty mocks, so
// any call it doesn't mock is named.
export const mockClient = (services: MockServices): FrontendClient => {
  const registry = new ServiceRegistry();
  for (const field of Object.keys(registrars) as (keyof ServiceTypes)[]) {
    (registrars[field] as Registrar<object>)(registry, complete(field, services[field] ?? {}));
  }
  const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
  new Host(a, b, registry);
  return new FrontendClient(b, a);
};

const MockServicesContext = createContext<MockServices>({});

// The services of the nearest MockApi (its parents' included).
export const useMockServices = (): MockServices => useContext(MockServicesContext);

// The nearest MockApi's service for a field, as the story's own type.
export const useMockService = <T,>(field: keyof ServiceTypes): T =>
  useContext(MockServicesContext)[field] as T;

export interface MockApiProps {
  /**
   * The services, or a function giving them, read once at mount (key the
   * MockApi to swap them). A function keeps mocks that start timers from
   * being made again on each render.
   */
  services: MockServices | (() => MockServices);
  children: ReactNode;
}

// Provides a FrontendClient over `services` merged onto the nearest outer
// MockApi's, so nested ones add services (MockNotifications does). Services
// it was given with a destroy() are destroyed when it unmounts.
export const MockApi: React.FC<MockApiProps> = ({ services, children }) => {
  const parent = useContext(MockServicesContext);
  const [[own, merged, client]] = useState(() => {
    const own = typeof services === "function" ? services() : services;
    const merged = { ...parent, ...own };
    return [own, merged, mockClient(merged)] as const;
  });

  useEffect(
    () => () => {
      for (const service of Object.values(own)) {
        (service as { destroy?: () => void })?.destroy?.();
      }
    },
    []
  );

  return (
    <MockServicesContext.Provider value={merged}>
      <ApiProvider value={client}>{children}</ApiProvider>
    </MockServicesContext.Provider>
  );
};
