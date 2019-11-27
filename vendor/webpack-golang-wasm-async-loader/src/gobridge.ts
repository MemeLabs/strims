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

function sleep() {
  return new Promise(requestAnimationFrame);
}

export default function(getBytes: Promise<Buffer>) {
  let ready = false;

  async function init() {
    const go = new g.Go();
    const bytes = await getBytes;
    const result = await WebAssembly.instantiate(bytes, go.importObject);
    go.run(result.instance);
    ready = true;
  }

  init();

  const proxy = new Proxy(
    {},
    {
      get: (_, key) => {
        return (...args: any) => {
          return new Promise(async (resolve, reject) => {
            const run = () => {
              const cb = (err: any, ...msg: any[]) => (err ? reject(err) : resolve(...msg));
              bridge[key].apply(undefined, [...args, cb]);
            };

            while (!ready) {
              await sleep();
            }

            if (!(key in bridge)) {
              reject(`There is nothing defined with the name "${key.toString()}"`);
              return;
            }

            if (typeof bridge[key] !== "function") {
              resolve(bridge[key]);
              return;
            }

            run();
          });
        };
      },
    },
  );

  return proxy;
}
