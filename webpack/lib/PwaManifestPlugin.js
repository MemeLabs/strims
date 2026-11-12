"use strict";
// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
Object.defineProperty(exports, "__esModule", { value: true });
exports.PwaManifestPlugin = void 0;
const crypto_1 = require("crypto");
const fs_1 = require("fs");
const webpack_1 = require("webpack");
const { RawSource } = webpack_1.sources;
const md5 = (data) => (0, crypto_1.createHash)("md5").update(data).digest("hex");
const attr = (value) => value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
/**
 * Emits a web app manifest, manifest.<md5>.json, and its icons,
 * icon_<size>x<size>.<md5>.png, and puts iOS's web app metas and a link to
 * the manifest at the end of every HtmlWebpackPlugin page's head but the
 * excluded ones.
 */
class PwaManifestPlugin {
    htmlWebpackPlugin;
    options;
    constructor(htmlWebpackPlugin, options) {
        this.htmlWebpackPlugin = htmlWebpackPlugin;
        this.options = options;
    }
    apply(compiler) {
        // Not the child compilations' (HtmlWebpackPlugin's templates among them).
        compiler.hooks.thisCompilation.tap("PwaManifestPlugin", (compilation) => {
            const { publicPath } = compilation.outputOptions;
            const base = typeof publicPath === "string" && publicPath !== "auto" ? publicPath : "";
            const icons = this.options.icons.map(({ src, size }) => {
                compilation.fileDependencies.add(src);
                const data = (0, fs_1.readFileSync)(src);
                const sizes = `${size}x${size}`;
                return { file: `icon_${sizes}.${md5(data)}.png`, sizes, data };
            });
            const json = JSON.stringify({
                icons: icons.map(({ file, sizes }) => ({ src: base + file, sizes, type: "image/png" })),
                ...this.options.manifest,
            }, null, 2);
            const manifestFile = `manifest.${md5(json)}.json`;
            compilation.hooks.processAssets.tap({ name: "PwaManifestPlugin", stage: webpack_1.Compilation.PROCESS_ASSETS_STAGE_ADDITIONAL }, () => {
                for (const { file, data } of icons) {
                    compilation.emitAsset(file, new RawSource(data), { immutable: true });
                }
                compilation.emitAsset(manifestFile, new RawSource(json), { immutable: true });
            });
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
exports.PwaManifestPlugin = PwaManifestPlugin;
