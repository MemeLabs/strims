// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import HtmlWebpackPlugin from "html-webpack-plugin";
import { Compiler, sources } from "webpack";

const { RawSource } = sources;

interface Options {
  chunk: string;
  varName: string;
  htmlFilename: string;
}

export class ChunkManifestPlugin {
  constructor(private htmlWebpackPlugin: typeof HtmlWebpackPlugin, private options: Options) {}

  apply(compiler: Compiler) {
    compiler.hooks.compilation.tap("ChunkManifestPlugin", (compilation) => {
      this.htmlWebpackPlugin
        .getHooks(compilation)
        .alterAssetTagGroups.tap("ChunkManifestPlugin", (assets) => {
          if (this.options.htmlFilename !== assets.outputName) {
            return assets;
          }

          const chunk = compilation.namedChunks.get(this.options.chunk);
          if (!chunk) {
            throw new Error(`chunk named "${this.options.chunk}" does not exist`);
          }
          const filename = `${this.options.chunk}.${chunk.renderedHash}.manifest.js`;
          const files = Array.from(chunk.getAllAsyncChunks())
            .map((c) => Array.from(c.files))
            .flat();
          compilation.emitAsset(
            filename,
            new RawSource(`const ${this.options.varName}=${JSON.stringify(files)};`)
          );

          const scriptIndex = assets.headTags.findIndex((t) => t.tagName === "script");
          const script = this.htmlWebpackPlugin.createHtmlTagObject("script", {
            defer: true,
            src: "/" + filename,
          });
          assets.headTags.splice(scriptIndex, 0, script);

          return assets;
        });
    });
  }
}
