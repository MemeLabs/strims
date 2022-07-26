"use strict";
// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only
exports.__esModule = true;
exports.ChunkManifestPlugin = void 0;
var webpack_1 = require("webpack");
var RawSource = webpack_1.sources.RawSource;
var ChunkManifestPlugin = /** @class */ (function () {
    function ChunkManifestPlugin(htmlWebpackPlugin, options) {
        this.htmlWebpackPlugin = htmlWebpackPlugin;
        this.options = options;
    }
    ChunkManifestPlugin.prototype.apply = function (compiler) {
        var _this = this;
        compiler.hooks.compilation.tap("ChunkManifestPlugin", function (compilation) {
            _this.htmlWebpackPlugin
                .getHooks(compilation)
                .alterAssetTagGroups.tap("ChunkManifestPlugin", function (assets) {
                if (_this.options.htmlFilename !== assets.outputName) {
                    return assets;
                }
                var chunk = compilation.namedChunks.get(_this.options.chunk);
                if (!chunk) {
                    throw new Error("chunk named \"".concat(_this.options.chunk, "\" does not exist"));
                }
                var filename = "".concat(_this.options.chunk, ".").concat(chunk.renderedHash, ".manifest.js");
                var files = Array.from(chunk.getAllAsyncChunks())
                    .map(function (c) { return Array.from(c.files); })
                    .flat();
                compilation.emitAsset(filename, new RawSource("const ".concat(_this.options.varName, "=").concat(JSON.stringify(files), ";")));
                var scriptIndex = assets.headTags.findIndex(function (t) { return t.tagName === "script"; });
                var script = _this.htmlWebpackPlugin.createHtmlTagObject("script", {
                    defer: true,
                    src: "/" + filename
                });
                assets.headTags.splice(scriptIndex, 0, script);
                return assets;
            });
        });
    };
    return ChunkManifestPlugin;
}());
exports.ChunkManifestPlugin = ChunkManifestPlugin;
