import * as React from "react";

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

const defaultOptions = {
  skip: false,
};

const create = <C extends Client>() => {
  const ClientContext = React.createContext<C>(null);

  const { Provider } = ClientContext;

  const useClient = () => React.useContext(ClientContext);

  const useCall = <S extends keyof C, M extends FunctionPropertyNames<C[S]>>(
    serviceName: S,
    methodName: M,
    options: Options<C, S, M> = {}
  ) => {
    type Arguments = Parameters<C[S][M]>;
    type Result = ResultType<C[S][M]>;
    type CallResult = ReturnType<C[S][M]>;
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
        throw new Error(`undefined api method ${serviceName as string}.${methodName as string}`);
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

  const useLazyCall = <S extends keyof C, M extends FunctionPropertyNames<C[S]>>(
    serviceName: S,
    methodName: M,
    options: Options<C, S, M> = {}
  ) => {
    return useCall(serviceName, methodName, { ...options, skip: true });
  };

  return {
    ClientContext,
    Provider,
    useClient,
    useCall,
    useLazyCall,
  };
};

export default create;
