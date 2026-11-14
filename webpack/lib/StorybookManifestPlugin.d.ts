import { Compiler } from "webpack";
/** The manifest's form and how its hashes are taken: hack/storybook/check.py
 * compares two builds' manifests only when they agree on it. */
export declare const MANIFEST_VERSION = 1;
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
export declare class StorybookManifestPlugin {
    private options;
    constructor(options: Options);
    apply(compiler: Compiler): void;
    private manifest;
}
export {};
