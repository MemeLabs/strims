import HtmlWebpackPlugin from "html-webpack-plugin";
import { Compiler } from "webpack";
interface Options {
    chunk: string;
    varName: string;
    htmlFilename: string;
}
export declare class ChunkManifestPlugin {
    private htmlWebpackPlugin;
    private options;
    constructor(htmlWebpackPlugin: typeof HtmlWebpackPlugin, options: Options);
    apply(compiler: Compiler): void;
}
export {};
