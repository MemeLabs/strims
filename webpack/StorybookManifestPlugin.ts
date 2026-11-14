// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { createHash } from "crypto";

import { Chunk, ChunkGroup, Compilation, Compiler, Module, NormalModule } from "webpack";

const NAME = "StorybookManifestPlugin";

/** The manifest's form and how its hashes are taken: hack/storybook/check.py
 * compares two builds' manifests only when they agree on it. */
export const MANIFEST_VERSION = 1;

interface Options {
  /** The entry a storybook frame document loads (devtools). */
  entry: string;
  /** The frame's module (src/devtools/storybook/host/Frame.tsx), which the
   * entry loads by import() in a frame document. */
  frame: string;
  /** The catalog (src/devtools/storybook/host/catalog.ts): its
   * require.contexts list every story module and every UI file. */
  catalog: string;
  /** Each value that names the build itself (its commit, its build time),
   * by the placeholder it is hashed as. */
  identity: Record<string, string>;
  /** The manifest's file name in the output. */
  filename: string;
}

/**
 * What each story's frame document loads, as hashes of webpack's output by
 * module, written to `filename` (storybook-manifest.json). A story whose
 * inputs, `shared` and its module's `stories` entry, hash the same in two
 * builds renders the same in both, so hack/storybook/check.py --compare
 * needn't capture it.
 *
 * `inputs` is [key, sha256] sorted by key: each module's generated code by
 * its id (a module generated for a worker's runtime too adds " (runtime)"),
 * each runtime module's as "runtime <name>", each output file no chunk owns
 * (devtools.html, locales/, emoji/) as "file <name>", and "webpack" (its
 * version, for the code it wraps chunks in). `shared` indexes the inputs
 * every frame loads: the entry's chunks, the frame's chunk group and the
 * groups below it but the stories', the files no chunk owns and webpack's
 * version. `stories` indexes, by module path as the storybook index names it
 * (chat/Chat), the inputs of that module's chunk group and every group below
 * it, workers' among them. `files` lists the output files each of the two
 * loads, which check.py holds the requests a capture makes to.
 *
 * A hash is of the code with the build's root directory as `<root>` (React's
 * development transform puts each file's absolute path in its code), its
 * identity values as their placeholders (the index's commit and build time,
 * i18n's cache-busting query), and each load of chunks (an import()'s, a
 * worker's startup's) as one of `<chunk>`, so two builds of one tree in two
 * directories hash the same: how splitChunks cuts the chunks hangs on module
 * sizes, and the modules a load brings are inputs of their own. Two things
 * are left out by name: the runtime's map of chunk file names, which says
 * where each chunk's file is (the chunks are inputs of their own), and the
 * catalog's two contexts' maps, which list every story module and UI file,
 * so a story added changes neither. A route stays its story's because
 * buildIndex checks that each route resolves to it.
 * Development builds name modules by path and neither shake nor concatenate
 * them, so a module's code is its own and its imports' paths.
 *
 * When the frame's group or the catalog's contexts can't be found, the
 * manifest says why in `error` and the build goes on.
 */
export class StorybookManifestPlugin {
  constructor(private options: Options) {}

  apply(compiler: Compiler) {
    compiler.hooks.thisCompilation.tap(NAME, (compilation) => {
      compilation.hooks.processAssets.tap(
        { name: NAME, stage: compiler.webpack.Compilation.PROCESS_ASSETS_STAGE_REPORT },
        () => {
          let manifest: object;
          try {
            manifest = this.manifest(compiler, compilation);
          } catch (e) {
            manifest = {
              version: MANIFEST_VERSION,
              error: e instanceof Error ? e.message : String(e),
            };
          }
          compilation.emitAsset(
            this.options.filename,
            new compiler.webpack.sources.RawSource(JSON.stringify(manifest))
          );
        }
      );
    });
  }

  private manifest(compiler: Compiler, compilation: Compilation) {
    const { webpack } = compiler;
    const { chunkGraph, moduleGraph, codeGenerationResults } = compilation;
    const { getRuntimeKey } = webpack.util.runtime;

    const placeholders: [string, string][] = [
      [compiler.context, "<root>"],
      ...Object.entries(this.options.identity).map(([p, v]): [string, string] => [v, p]),
    ];
    const normalize = (text: string) =>
      placeholders.reduce((s, [value, placeholder]) => s.split(value).join(placeholder), text);
    // Which chunks an import() loads, and their ids, hang on how splitChunks
    // cut them, which module sizes decide (a longer root makes every module
    // longer): its code loads <chunk>, whatever chunks, as many as they are.
    const chunkIds = new Set([...compilation.chunks].map((c) => String(c.id)));
    const load = "__webpack_require__.e(<chunk>)";
    const unchunk = (code: string) =>
      code
        .replace(
          /__webpack_require__\.e\(\s*(?:\/\*.*?\*\/\s*)?"([^"\\]*)"\)/g,
          (m: string, id: string) => (chunkIds.has(id) ? load : m)
        )
        .replace(
          /Promise\.all\(\s*(?:\/\*.*?\*\/\s*)?\[(?:__webpack_require__\.e\(<chunk>\)(?:,\s*)?)+\]\)/g,
          load
        )
        .replace(/Promise\.resolve\(\s*\/\*! import\(\)[^*]*\*\/\s*\)/g, load)
        .replace(/"([^"\\\n]*)"/g, (m: string, id: string) => (chunkIds.has(id) ? '"<chunk>"' : m))
        .replace(/"<chunk>"(?:,\s*"<chunk>")+/g, '"<chunk>"');
    const code = (text: string) => unchunk(normalize(text));
    const sha = (data: string | Buffer) => createHash("sha256").update(data).digest("hex");

    const entry = compilation.entrypoints.get(this.options.entry);
    if (!entry) {
      throw new Error(`no entry named ${this.options.entry}`);
    }
    const mainRuntime = getRuntimeKey(entry.getRuntimeChunk().runtime);
    const byResource = (resource: string) =>
      [...compilation.modules].find((m) => (m as NormalModule).resource === resource);
    const frame = byResource(this.options.frame);
    const catalog = byResource(this.options.catalog);
    if (!frame || !catalog) {
      throw new Error(`no module ${frame ? this.options.catalog : this.options.frame}`);
    }
    const contexts = new Set(
      [...compilation.modules].filter(
        (m) => m.type === "javascript/dynamic" && moduleGraph.getIssuer(m) === catalog
      )
    );
    const lazy = [...contexts].filter(
      (m) => (m as unknown as { options?: { mode?: string } }).options?.mode === "lazy"
    );
    if (contexts.size !== 2 || lazy.length !== 1) {
      throw new Error(
        `the catalog's contexts: ${contexts.size}, lazy ${lazy.length} (expected 2, one lazy)`
      );
    }

    // Each stories module's chunk group, by the request the lazy context
    // loads it by ("./chat/Chat.stories.tsx": chat/Chat).
    const storyGroups = new Map<string, ChunkGroup>();
    for (const group of compilation.chunkGroups) {
      for (const origin of group.origins) {
        if (origin.module === lazy[0] && origin.request?.startsWith("./")) {
          storyGroups.set(origin.request.replace(/^\.\/|\.stories\.tsx$/g, ""), group);
        }
      }
    }
    const frameGroups = [...entry.childrenIterable].filter((g) =>
      g.chunks.some((c) => chunkGraph.isModuleInChunk(frame, c))
    );
    if (frameGroups.length !== 1) {
      throw new Error(`the frame's chunk groups below the entry: ${frameGroups.length}`);
    }

    const inputs = new Map<string, string>();
    const ownedFiles = new Set<string>();
    for (const chunk of compilation.chunks) {
      for (const f of [...chunk.files, ...chunk.auxiliaryFiles]) {
        ownedFiles.add(f);
      }
    }

    const moduleHash = (module: Module, chunk: Chunk) => {
      if (contexts.has(module)) {
        return "left out: the catalog's list of every story module and UI file";
      }
      const result = codeGenerationResults.get(module, chunk.runtime);
      const h = createHash("sha256");
      for (const type of [...result.sources.keys()].sort()) {
        const src = result.sources.get(type).source();
        h.update(`${type}\0`);
        h.update(typeof src === "string" ? code(src) : src);
        h.update("\0");
      }
      return h.digest("hex");
    };

    // The inputs and files of a chunk's modules, runtime modules and output.
    const addChunk = (chunk: Chunk, keys: Set<string>, files: Set<string>) => {
      const runtime = getRuntimeKey(chunk.runtime);
      const suffix = runtime === mainRuntime ? "" : ` (${runtime})`;
      for (const module of chunkGraph.getChunkModulesIterable(chunk)) {
        const key = `${chunkGraph.getModuleId(module)}${suffix}`;
        if (!inputs.has(key)) {
          inputs.set(key, moduleHash(module, chunk));
        }
        keys.add(key);
      }
      for (const module of chunkGraph.getChunkRuntimeModulesIterable(chunk)) {
        const key = `runtime ${module.name}${suffix}`;
        if (!inputs.has(key)) {
          inputs.set(
            key,
            module instanceof webpack.runtime.GetChunkFilenameRuntimeModule
              ? "left out: where each chunk's file is"
              : sha(code(module.getGeneratedCode() ?? ""))
          );
        }
        keys.add(key);
      }
      for (const f of [...chunk.files, ...chunk.auxiliaryFiles]) {
        if (!f.endsWith(".map")) {
          files.add(f);
        }
      }
    };

    // The groups below `start` (children and workers' entrypoints), but `skip`.
    const below = (start: ChunkGroup, skip: Set<ChunkGroup>) => {
      const seen = new Set<ChunkGroup>();
      const todo = [start];
      while (todo.length) {
        const g = todo.pop();
        if (seen.has(g) || skip.has(g)) {
          continue;
        }
        seen.add(g);
        todo.push(...g.childrenIterable, ...g.asyncEntrypointsIterable);
      }
      return seen;
    };
    const scope = (groups: Iterable<ChunkGroup>, chunks: Iterable<Chunk> = []) => {
      const keys = new Set<string>();
      const files = new Set<string>();
      for (const chunk of chunks) {
        addChunk(chunk, keys, files);
      }
      for (const g of groups) {
        for (const chunk of g.chunks) {
          addChunk(chunk, keys, files);
        }
      }
      return { keys, files };
    };

    const shared = scope(below(frameGroups[0], new Set(storyGroups.values())), entry.chunks);
    const chunkFiles = [...ownedFiles]
      .filter((f) => !f.endsWith(".map"))
      .sort((a, b) => b.length - a.length);
    for (const asset of compilation.getAssets()) {
      if (
        ownedFiles.has(asset.name) ||
        asset.name.endsWith(".map") ||
        asset.name === this.options.filename
      ) {
        continue;
      }
      // A page names the chunk files it loads, which are inputs of their own.
      const bytes = asset.source.buffer();
      const key = `file ${asset.name}`;
      inputs.set(
        key,
        asset.name.endsWith(".html")
          ? sha(
              chunkFiles.reduce(
                (s, f) => s.split(f).join("<chunk file>"),
                normalize(bytes.toString())
              )
            )
          : sha(bytes)
      );
      shared.keys.add(key);
      shared.files.add(asset.name);
    }
    inputs.set("webpack", sha(webpack.version));
    shared.keys.add("webpack");

    const stories = new Map(
      [...storyGroups].map(([path, g]) => [path, scope(below(g, new Set()))])
    );

    const keys = [...inputs.keys()].sort();
    const index = new Map(keys.map((k, i) => [k, i]));
    const indices = (s: Set<string>) => [...s].map((k) => index.get(k)).sort((a, b) => a - b);
    const sorted = <T>(m: Map<string, T>) =>
      [...m].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
    return {
      version: MANIFEST_VERSION,
      inputs: keys.map((k) => [k, inputs.get(k)]),
      shared: indices(shared.keys),
      stories: Object.fromEntries(sorted(stories).map(([p, s]) => [p, indices(s.keys)])),
      files: {
        shared: [...shared.files].sort(),
        stories: Object.fromEntries(sorted(stories).map(([p, s]) => [p, [...s.files].sort()])),
      },
    };
  }
}
