declare global {
  interface Window {
    __gobridge__: any

    Go: any
  }
}

const g = global || window || self;

if (!g.__gobridge__) {
  g.__gobridge__ = {};
}

const bridge = g.__gobridge__;

const caller = (name: string) => (...args: any) => new Promise((resolve, reject) => {
  const cb = (err: any, ...msg: any[]) => (err ? reject(err) : resolve(...msg));
  bridge[name].apply(undefined, [ ...args, cb ]);
});

export default function(wasmPath: string) {
  return async (baseURI: string, wasmio: unknown) => {
    const go = new g.Go();
    go.wasmio = wasmio;

    const bytes = await fetch(`${baseURI}/${wasmPath}`).then(res => res.arrayBuffer());
    const result = await WebAssembly.instantiate(bytes, go.importObject);
    go.run(result.instance);

    return Object.fromEntries(Object.keys(bridge).map((k) => [ k, caller(k) ]));
  };
}
