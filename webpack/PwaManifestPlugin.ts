// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { createHash } from "crypto";
import { readFileSync } from "fs";

import HtmlWebpackPlugin from "html-webpack-plugin";
import { Compilation, Compiler, sources } from "webpack";

const { RawSource } = sources;

interface Icon {
  /** A square PNG file. */
  src: string;
  /** Its width and height in pixels. */
  size: number;
}

interface Options {
  /** The web app manifest's members besides its icons, in its order. */
  manifest: { name: string } & Record<string, unknown>;
  /** The manifest's icons, in its order. */
  icons: Icon[];
  /** HtmlWebpackPlugin pages (their output names) left without the tags. */
  exclude?: string[];
}

const md5 = (data: Buffer | string) => createHash("md5").update(data).digest("hex");

const attr = (value: string) =>
  value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");

/**
 * Emits a web app manifest, manifest.<md5>.json, and its icons,
 * icon_<size>x<size>.<md5>.png, and puts iOS's web app metas and a link to
 * the manifest at the end of every HtmlWebpackPlugin page's head but the
 * excluded ones.
 */
export class PwaManifestPlugin {
  constructor(private htmlWebpackPlugin: typeof HtmlWebpackPlugin, private options: Options) {}

  apply(compiler: Compiler) {
    // Not the child compilations' (HtmlWebpackPlugin's templates among them).
    compiler.hooks.thisCompilation.tap("PwaManifestPlugin", (compilation) => {
      const { publicPath } = compilation.outputOptions;
      const base = typeof publicPath === "string" && publicPath !== "auto" ? publicPath : "";

      const icons = this.options.icons.map(({ src, size }) => {
        compilation.fileDependencies.add(src);
        const data = readFileSync(src);
        const sizes = `${size}x${size}`;
        return { file: `icon_${sizes}.${md5(data)}.png`, sizes, data };
      });
      const json = JSON.stringify(
        {
          icons: icons.map(({ file, sizes }) => ({ src: base + file, sizes, type: "image/png" })),
          ...this.options.manifest,
        },
        null,
        2
      );
      const manifestFile = `manifest.${md5(json)}.json`;

      compilation.hooks.processAssets.tap(
        { name: "PwaManifestPlugin", stage: Compilation.PROCESS_ASSETS_STAGE_ADDITIONAL },
        () => {
          for (const { file, data } of icons) {
            compilation.emitAsset(file, new RawSource(data), { immutable: true });
          }
          compilation.emitAsset(manifestFile, new RawSource(json), { immutable: true });
        }
      );

      const tags = [
        `<meta name="apple-mobile-web-app-title" content="${attr(this.options.manifest.name)}" />`,
        `<meta name="apple-mobile-web-app-capable" content="yes" />`,
        `<meta name="apple-mobile-web-app-status-bar-style" content="default" />`,
        `<link rel="manifest" href="${attr(base + manifestFile)}" />`,
      ].join("");
      this.htmlWebpackPlugin.getHooks(compilation).beforeEmit.tap("PwaManifestPlugin", (data) => {
        if (!this.options.exclude?.includes(data.outputName)) {
          data.html = data.html.replace(/<\/head>/i, (head) => tags + head);
        }
        return data;
      });
    });
  }
}
