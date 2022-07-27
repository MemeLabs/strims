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
    const res = await fetch(baseURI + wasmPath);
    const mod = await WebAssembly.compileStreaming(res);

    // we need to run wasm startup in a retry loop because in ios wasm's stack
    // size is not predictable (https://bugs.webkit.org/show_bug.cgi?id=201028).
    // this bug manifests nondeterministically. the bug comments suggest adding
    // a delay before running the instance but this doesn't work reliably.

    // when startup fails the process exits resolving go.run(). because the bug
    // only seems to affect init functions we can mark the process live in the
    // main function to exit the loop as soon as we have a viable instance.

    // in testing this resolved within 5 retries but the bug report suggests the
    // behavior varies across devices...
    for (let i = 0; i < 10; i ++) {
      try {
        const go = Object.assign(new g.Go(), { wasmio });
        const instance = await WebAssembly.instantiate(mod, go.importObject);
        await Promise.race([
          new Promise<void>((resolve) => { bridge.__markLive__ = resolve; }),
          go.run(instance),
        ]);
        break;
      } catch(e) {
        console.error(e);
      }
    }

    return Object.fromEntries(Object.keys(bridge).map((k) => [ k, caller(k) ]));
  };
}
