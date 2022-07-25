"use strict";
exports.__esModule = true;
exports.ChunkManifestPlugin = void 0;
var webpack_1 = require("webpack");
var RawSource = webpack_1.sources.RawSource;
var getManifest = function (chunk) {
    var summary = { files: Array.from(chunk.files.values()) };
    var asyncChunks = Array.from(chunk.getAllAsyncChunks().values());
    if (asyncChunks.length) {
        summary.asyncChunks = asyncChunks.map(function (c) { return getManifest(c); });
    }
    return summary;
};
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
                compilation.emitAsset(filename, new RawSource("const ".concat(_this.options.varName, "=").concat(JSON.stringify(getManifest(chunk)), ";")));
                assets.headTags.unshift({
                    tagName: "script",
                    voidTag: false,
                    meta: { plugin: "chunk-manifest-plugin" },
                    attributes: {
                        defer: true,
                        type: undefined,
                        src: "/" + filename
                    }
                });
                return assets;
            });
        });
    };
    return ChunkManifestPlugin;
}());
exports.ChunkManifestPlugin = ChunkManifestPlugin;
