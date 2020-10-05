import * as React from "react";

import Client from "../lib/api/frontendRPCClient";

export const ClientContext = React.createContext<Client>(null);

export const { Provider } = ClientContext;

type AnyFunction = (...arg: any) => any;
type FunctionPropertyNames<T> = { [K in keyof T]: T[K] extends AnyFunction ? K : never }[keyof T];
type ResultType<T extends AnyFunction> = ReturnType<T> extends Promise<infer U> ? U : ReturnType<T>;

type ClientMethodName = FunctionPropertyNames<Client>;

export interface Options<T extends ClientMethodName> {
  skip?: boolean;
  args?: Parameters<Client[T]>;
  onComplete?: (data: ResultType<Client[T]>) => void;
  onError?: (error: Error) => void;
}

const defaultOptions = {
  skip: false,
};

export const useClient = () => React.useContext(ClientContext);

export const useCall = <T extends ClientMethodName>(methodName: T, options: Options<T> = {}) => {
  type Arguments = Parameters<Client[T]>;
  type Result = ResultType<Client[T]>;
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

  const call = (...args: Arguments) => {
    /* eslint-disable prefer-spread */
    const value = client[methodName].apply(client, args);
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
  };

  React.useEffect(() => {
    if (!options.skip) {
      call.apply(this, options.args || []);
    }
  }, [options.skip]);

  return [state, call] as [State, (...arg: Arguments) => void];
};

export const useLazyCall = <T extends ClientMethodName>(
  methodName: T,
  options: Options<T> = {}
) => {
  return useCall(methodName, { ...options, skip: true });
};
