"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const g = global || window || self;
if (!g.__gobridge__) {
    g.__gobridge__ = {};
}
const bridge = g.__gobridge__;
const caller = (name) => (...args) => new Promise((resolve, reject) => {
    const cb = (err, ...msg) => (err ? reject(err) : resolve(...msg));
    bridge[name].apply(undefined, [...args, cb]);
});
function default_1(wasmPath) {
    return async (baseURI, wasmio) => {
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
        for (let i = 0; i < 10; i++) {
            try {
                const go = Object.assign(new g.Go(), { wasmio });
                const instance = await WebAssembly.instantiate(mod, go.importObject);
                await Promise.race([
                    new Promise((resolve) => { bridge.__markLive__ = resolve; }),
                    go.run(instance),
                ]);
                break;
            }
            catch (e) {
                console.error(e);
            }
        }
        return Object.fromEntries(Object.keys(bridge).map((k) => [k, caller(k)]));
    };
}
exports.default = default_1;
//# sourceMappingURL=gobridge.js.map