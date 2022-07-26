// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import HtmlWebpackPlugin from "html-webpack-plugin";
import { Chunk, Compiler, sources } from "webpack";

const { RawSource } = sources;

interface ChunkManifest {
  files: string[];
  asyncChunks?: ChunkManifest[];
}

const getManifest = (chunk: Chunk): ChunkManifest => {
  const summary: ChunkManifest = { files: Array.from(chunk.files.values()) };

  const asyncChunks = Array.from(chunk.getAllAsyncChunks().values());
  if (asyncChunks.length) {
    summary.asyncChunks = asyncChunks.map((c) => getManifest(c));
  }

  return summary;
};

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
          compilation.emitAsset(
            filename,
            new RawSource(`const ${this.options.varName}=${JSON.stringify(getManifest(chunk))};`)
          );

          assets.headTags.unshift({
            tagName: "script",
            voidTag: false,
            meta: { plugin: "chunk-manifest-plugin" },
            attributes: {
              defer: true,
              type: undefined,
              src: "/" + filename,
            },
          });
          return assets;
        });
    });
  }
}
