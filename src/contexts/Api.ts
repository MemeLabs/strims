import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React, { createContext, useContext, useEffect, useMemo, useRef } from "react";

import type { Client } from "../apis/client";

type AnyFunction = (...arg: any) => any;
type FunctionPropertyNames<T> = { [K in keyof T]: T[K] extends AnyFunction ? K : never }[keyof T];
type ResultType<T extends AnyFunction> = ReturnType<T> extends Promise<infer U> ? U : ReturnType<T>;

type MethodParameters<T> = T extends AnyFunction ? Parameters<T> : never;
type MethodResultType<T> = T extends AnyFunction ? ResultType<T> : never;
type MethodReturnType<T> = T extends AnyFunction ? ReturnType<T> : never;

export interface Options<
  C extends Client,
  S extends keyof C,
  M extends FunctionPropertyNames<C[S]>
> {
  skip?: boolean;
  args?: MethodParameters<C[S][M]>;
  onComplete?: (data: MethodResultType<C[S][M]>) => void;
  onError?: (error: Error) => void;
}

export interface CallHookState<
  C extends Client,
  S extends keyof C,
  M extends FunctionPropertyNames<C[S]>
> {
  value?: MethodResultType<C[S][M]>;
  error?: Error;
  loading: boolean;
  called: boolean;
}

export type CallHookDispatcher<
  C extends Client,
  S extends keyof C,
  M extends FunctionPropertyNames<C[S]>
> = (...arg: MethodParameters<C[S][M]>) => MethodReturnType<C[S][M]>;

export type CallHook<C extends Client> = <S extends keyof C, M extends FunctionPropertyNames<C[S]>>(
  serviceName: S,
  methodName: M,
  options?: Options<C, S, M>
) => [CallHookState<C, S, M>, CallHookDispatcher<C, S, M>];

export interface Api<C extends Client> {
  ClientContext: React.Context<C>;
  Provider: React.Provider<C>;
  useClient: () => C;
  useCall: CallHook<C>;
  useLazyCall: CallHook<C>;
}

const defaultOptions = {
  skip: false,
};

const create = <C extends Client>(): Api<C> => {
  const ClientContext = createContext<C>(null);

  const { Provider } = ClientContext;

  const useClient = () => useContext(ClientContext);

  const useCall: CallHook<C> = <S extends keyof C, M extends FunctionPropertyNames<C[S]>>(
    serviceName: S,
    methodName: M,
    options: Options<C, S, M> = {}
  ) => {
    options = { ...defaultOptions, ...options };

    const client = useContext(ClientContext);
    const [state, setState] = React.useState<CallHookState<C, S, M>>({
      loading: !options.skip,
      called: !options.skip,
    });

    const mounted = useRef(true);
    useEffect(() => () => void (mounted.current = false), []);

    const handleError = (error: Error) => {
      if (!mounted.current) {
        return;
      }
      setState((prev) => ({
        ...prev,
        loading: false,
        value: undefined,
        error,
      }));
      if (options.onError) {
        options.onError(error);
      }
    };

    const handleComplete = (value: MethodResultType<C[S][M]>) => {
      if (!mounted.current) {
        return;
      }
      setState((prev) => ({
        ...prev,
        called: true,
        loading: false,
        error: undefined,
        value,
      }));
      if (options.onComplete) {
        options.onComplete(value);
      }
    };

    const call: CallHookDispatcher<C, S, M> = (...args) => {
      const service = client[serviceName];
      const method = service?.[methodName];
      if (method === undefined) {
        throw new Error({
          message: `undefined api method ${serviceName as string}.${methodName as string}`,
        });
      }

      // eslint-disable-next-line
      const value = (method as any).apply(service, args) as MethodResultType<C[S][M]>;
      if (value instanceof Promise) {
        setState((prev) => ({
          ...prev,
          loading: true,
          called: true,
        }));
        value.then(handleComplete).catch(handleError);
      } else {
        handleComplete(value);
      }
      return value as MethodReturnType<C[S][M]>;
    };

    useEffect(() => {
      if (!options.skip) {
        call.apply(this, options.args || []);
      }
    }, [options.skip]);

    return useMemo(() => [state, call], [serviceName, methodName, options]);
  };

  const useLazyCall: CallHook<C> = <S extends keyof C, M extends FunctionPropertyNames<C[S]>>(
    serviceName: S,
    methodName: M,
    options: Options<C, S, M> = {}
  ) => useCall(serviceName, methodName, { ...options, skip: true });

  return {
    ClientContext,
    Provider,
    useClient,
    useCall,
    useLazyCall,
  };
};

export default create;
