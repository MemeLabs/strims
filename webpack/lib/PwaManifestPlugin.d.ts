import HtmlWebpackPlugin from "html-webpack-plugin";
import { Compiler } from "webpack";
interface Icon {
    /** A square PNG file. */
    src: string;
    /** Its width and height in pixels. */
    size: number;
}
interface Options {
    /** The web app manifest's members besides its icons, in its order. */
    manifest: {
        name: string;
    } & Record<string, unknown>;
    /** The manifest's icons, in its order. */
    icons: Icon[];
    /** HtmlWebpackPlugin pages (their output names) left without the tags. */
    exclude?: string[];
}
/**
 * Emits a web app manifest, manifest.<md5>.json, and its icons,
 * icon_<size>x<size>.<md5>.png, and puts iOS's web app metas and a link to
 * the manifest at the end of every HtmlWebpackPlugin page's head but the
 * excluded ones.
 */
export declare class PwaManifestPlugin {
    private htmlWebpackPlugin;
    private options;
    constructor(htmlWebpackPlugin: typeof HtmlWebpackPlugin, options: Options);
    apply(compiler: Compiler): void;
}
export {};
