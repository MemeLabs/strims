import React from "react";

import type { Client } from "../apis/client";

type AnyFunction = (...arg: any) => any;
type FunctionPropertyNames<T> = { [K in keyof T]: T[K] extends AnyFunction ? K : never }[keyof T];
type ResultType<T extends AnyFunction> = ReturnType<T> extends Promise<infer U> ? U : ReturnType<T>;

export interface Options<
  C extends Client,
  S extends keyof C,
  M extends FunctionPropertyNames<C[S]>
> {
  skip?: boolean;
  args?: Parameters<C[S][M]>;
  onComplete?: (data: ResultType<C[S][M]>) => void;
  onError?: (error: Error) => void;
}

export interface CallHookState<
  C extends Client,
  S extends keyof C,
  M extends FunctionPropertyNames<C[S]>
> {
  value?: ResultType<C[S][M]>;
  error?: Error;
  loading: boolean;
  called: boolean;
}

export type CallHookDispatcher<
  C extends Client,
  S extends keyof C,
  M extends FunctionPropertyNames<C[S]>
> = (...arg: Parameters<C[S][M]>) => ReturnType<C[S][M]>;

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
  const ClientContext = React.createContext<C>(null);

  const { Provider } = ClientContext;

  const useClient = () => React.useContext(ClientContext);

  const useCall: CallHook<C> = <S extends keyof C, M extends FunctionPropertyNames<C[S]>>(
    serviceName: S,
    methodName: M,
    options: Options<C, S, M> = {}
  ) => {
    options = { ...defaultOptions, ...options };

    const client = React.useContext(ClientContext);
    const [state, setState] = React.useState<CallHookState<C, S, M>>({
      loading: !options.skip,
      called: !options.skip,
    });

    let mounted = true;
    React.useEffect(() => {
      return () => (mounted = false);
    }, []);

    const handleError = (error: Error) => {
      if (!mounted) {
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

    const handleComplete = (value: ResultType<C[S][M]>) => {
      if (!mounted) {
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
        throw new Error(`undefined api method ${serviceName as string}.${methodName as string}`);
      }

      // eslint-disable-next-line
      const value = (method as any).apply(service, args) as ResultType<C[S][M]>;
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
      return value as ReturnType<C[S][M]>;
    };

    React.useEffect(() => {
      if (!options.skip) {
        call.apply(this, options.args || []);
      }
    }, [options.skip]);

    return [state, call];
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
