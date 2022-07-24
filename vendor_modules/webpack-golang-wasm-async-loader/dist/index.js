"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const child_process_1 = require("child_process");
const crypto = require("crypto");
const fs_1 = require("fs");
const path_1 = require("path");
const os_1 = require("os");
const versionPkg = "github.com/MemeLabs/go-ppspp/pkg/version";
function loader(contents) {
    const cb = this.async();
    const opts = {
        env: {
            GOPATH: process.env.GOPATH,
            GOROOT: process.env.GOROOT,
            GOCACHE: path_1.join(os_1.homedir(), ".cache", "go-build"),
            GOMODCACHE: path_1.join(process.env.GOPATH, "pkg", "mod", "cache"),
            GOOS: "js",
            GOARCH: "wasm",
            GOWASM: "satconv,signext",
        },
    };
    const goBin = path_1.join(opts.env.GOROOT, "bin", "go");
    const rand = Math.ceil(Math.random() * Number.MAX_SAFE_INTEGER).toString(36);
    const outFile = this.resourcePath + `.${rand}.wasm`;
    const args = ["build", "-mod", "readonly"];
    if (this.mode === "production") {
        const rev = process.env.VERSION || child_process_1.execFileSync("git", ["rev-parse", "HEAD"]).toString().substr(0, 8);
        args.push("-trimpath", "-ldflags", `-s -w -X '${versionPkg}.Platform=web' -X '${versionPkg}.Version=${rev}'`);
    }
    else {
        args.push("-ldflags", `-X '${versionPkg}.Platform=web'`);
    }
    args.push("-o", outFile, this.resourcePath);
    child_process_1.execFile(goBin, args, opts, (err) => {
        if (err) {
            cb(err);
            return;
        }
        const out = fs_1.readFileSync(outFile);
        const hash = crypto.createHash("sha256");
        hash.write(out);
        const digest = hash.digest().toString("hex").substring(0, 20);
        fs_1.unlinkSync(outFile);
        const chunkNameBase = path_1.basename(this.resourcePath, ".go") + `.${digest}`;
        const wasmChunks = [];
        const chunkSize = 20000000;
        for (let i = 0; i < out.byteLength / chunkSize; i++) {
            const chunkName = chunkNameBase + `.${i}.wasm`;
            wasmChunks.push(chunkName);
            const chunk = out.subarray(i * chunkSize, Math.min((i + 1) * chunkSize, out.byteLength));
            this.emitFile(chunkName, chunk, null);
        }
        cb(null, [
            `require("${path_1.join(__dirname, "..", "lib", "wasm_exec.js")}");`,
            `import gobridge from "${path_1.join(__dirname, "..", "dist", "gobridge.js")}";`,
            `export const wasmChunks = ${JSON.stringify(wasmChunks)};`,
            `export default gobridge(wasmChunks);`,
        ].join("\n"));
    });
}
exports.default = loader;
//# sourceMappingURL=index.js.map