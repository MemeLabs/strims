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
        const go = new g.Go();
        go.wasmio = wasmio;
        const bytes = await fetch(`${baseURI}/${wasmPath}`).then(res => res.arrayBuffer());
        const result = await WebAssembly.instantiate(bytes, go.importObject);
        go.run(result.instance);
        return Object.fromEntries(Object.keys(bridge).map((k) => [k, caller(k)]));
    };
}
exports.default = default_1;
//# sourceMappingURL=gobridge.js.map