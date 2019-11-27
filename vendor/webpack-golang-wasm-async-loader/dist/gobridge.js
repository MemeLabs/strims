"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const g = global || window || self;
if (!g.__gobridge__) {
    g.__gobridge__ = {};
}
const bridge = g.__gobridge__;
function sleep() {
    return new Promise(requestAnimationFrame);
}
function default_1(getBytes) {
    let ready = false;
    async function init() {
        const go = new g.Go();
        let bytes = await getBytes;
        let result = await WebAssembly.instantiate(bytes, go.importObject);
        go.run(result.instance);
        ready = true;
    }
    init();
    let proxy = new Proxy({}, {
        get: (_, key) => {
            return (...args) => {
                return new Promise(async (resolve, reject) => {
                    let run = () => {
                        let cb = (err, ...msg) => (err ? reject(err) : resolve(...msg));
                        bridge[key].apply(undefined, [...args, cb]);
                    };
                    while (!ready) {
                        await sleep();
                    }
                    if (!(key in bridge)) {
                        reject(`There is nothing defined with the name "${key.toString()}"`);
                        return;
                    }
                    if (typeof bridge[key] !== 'function') {
                        resolve(bridge[key]);
                        return;
                    }
                    run();
                });
            };
        }
    });
    return proxy;
}
exports.default = default_1;
//# sourceMappingURL=gobridge.js.map