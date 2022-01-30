import { execFile, execFileSync } from "child_process";
import * as crypto from "crypto";
import { readFileSync, unlinkSync } from "fs";
import { basename, join } from "path";
import { homedir } from "os";

import * as webpack from "webpack";

const versionPkg = "github.com/MemeLabs/go-ppspp/pkg/version";

function loader(this: webpack.loader.LoaderContext, contents: string) {
  const cb = this.async();

  const opts = {
    env: {
      GOPATH: process.env.GOPATH,
      GOROOT: process.env.GOROOT,
      GOCACHE: join(homedir(), ".cache", "go-build"),
      GOMODCACHE: join(process.env.GOPATH, "pkg", "mod", "cache"),
      GOOS: "js",
      GOARCH: "wasm",
    },
  };

  const goBin = join(opts.env.GOROOT, "bin", "go");
  const outFile = this.resourcePath + ".wasm";

  const args = ["build", "-mod", "readonly"];
  if (this.mode === "production") {
    const rev = process.env.VERSION || execFileSync("git", ["rev-parse", "HEAD"]).toString().substr(0, 8);
    args.push(
      "-trimpath",
      "-ldflags",
      `-s -w -X '${versionPkg}.Platform=web' -X '${versionPkg}.Version=${rev}'`
    );
  } else {
    args.push("-ldflags", `-X '${versionPkg}.Platform=web'`);
  }
  args.push("-o", outFile, this.resourcePath);

  execFile(goBin, args, opts, (err) => {
    if (err) {
      cb(err);
      return;
    }

    const out = readFileSync(outFile);

    const hash = crypto.createHash("sha256");
    hash.write(out);
    const digest = hash.digest().toString("hex").substring(0, 20);

    unlinkSync(outFile);
    const emittedFilename = basename(this.resourcePath, ".go") + `.${digest}.wasm`;
    this.emitFile(emittedFilename, out, null);

    cb(null, [
      `require("${join(__dirname, "..", "lib", "wasm_exec.js")}");`,
      `import gobridge from "${join(__dirname, "..", "dist", "gobridge.js")}";`,
      `export default gobridge("${emittedFilename}");`,
    ].join("\n"));
  });
}

export default loader;
