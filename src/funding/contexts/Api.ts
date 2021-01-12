import * as React from "react";

import { FundingClient } from "../../apis/client";

export const ClientContext = React.createContext<FundingClient>(null);

export const { Provider } = ClientContext;

type AnyFunction = (...arg: any) => any;
type FunctionPropertyNames<T> = { [K in keyof T]: T[K] extends AnyFunction ? K : never }[keyof T];
type ResultType<T extends AnyFunction> = ReturnType<T> extends Promise<infer U> ? U : ReturnType<T>;

export interface Options<
  S extends keyof FundingClient,
  M extends FunctionPropertyNames<FundingClient[S]>
> {
  skip?: boolean;
  args?: Parameters<FundingClient[S][M]>;
  onComplete?: (data: ResultType<FundingClient[S][M]>) => void;
  onError?: (error: Error) => void;
}

const defaultOptions = {
  skip: false,
};

export const useClient = () => React.useContext(ClientContext);

export const useCall = <
  S extends keyof FundingClient,
  M extends FunctionPropertyNames<FundingClient[S]>
>(
  serviceName: S,
  methodName: M,
  options: Options<S, M> = {}
) => {
  type Arguments = Parameters<FundingClient[S][M]>;
  type Result = ResultType<FundingClient[S][M]>;
  type CallResult = ReturnType<FundingClient[S][M]>;
  interface State {
    value?: Result;
    error?: Error;
    loading: boolean;
    called: boolean;
  }

  options = { ...defaultOptions, ...options };

  const client = React.useContext(ClientContext);
  const [state, setState] = React.useState<State>({
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

  const handleComplete = (value: Result) => {
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

  const call = (...args: Arguments): CallResult => {
    /* eslint-disable prefer-spread */
    const service = client[serviceName];
    const method = service?.[methodName];
    if (method === undefined) {
      throw new Error(`undefined api method ${serviceName}.${methodName as string}`);
    }

    const value = (method as AnyFunction).apply(service, args);
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
    return value;
  };

  React.useEffect(() => {
    if (!options.skip) {
      call.apply(this, options.args || []);
    }
  }, [options.skip]);

  return [state, call] as [State, (...arg: Arguments) => CallResult];
};

export const useLazyCall = <
  S extends keyof FundingClient,
  M extends FunctionPropertyNames<FundingClient[S]>
>(
  serviceName: S,
  methodName: M,
  options: Options<S, M> = {}
) => {
  return useCall(serviceName, methodName, { ...options, skip: true });
};
