// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// hack/storybook/scope_test.py's storybook in miniature, built as the
// console target builds the real one (development mode, React's
// development transform, style-loader, asset files, content hashes,
// DefinePlugin's build identity), with webpack/StorybookManifestPlugin.ts.
// The test gives NODE_MODULES (the checkout's node_modules; NODE_PATH too,
// for this file's requires), MANIFEST_PLUGIN (the plugin's build) and
// --env COMMIT, TIME and MIN_SIZE (splitChunks' minSize, else webpack's
// own).
const path = require("path");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const webpack = require("webpack");

const { StorybookManifestPlugin } = require(process.env.MANIFEST_PLUGIN);

module.exports = (env) => ({
  mode: "development",
  context: __dirname,
  entry: { devtools: "./src/index.tsx" },
  devtool: "source-map",
  output: {
    filename: "[name].[contenthash].js",
    chunkFilename: "[id].[contenthash].js",
    path: path.join(__dirname, "dist"),
    publicPath: "/",
    clean: true,
  },
  optimization: {
    realContentHash: true,
    ...(env.MIN_SIZE === undefined ? {} : { splitChunks: { minSize: Number(env.MIN_SIZE) } }),
  },
  resolve: {
    extensions: [".tsx", ".ts", ".js"],
    modules: ["node_modules", process.env.NODE_MODULES],
  },
  resolveLoader: { modules: ["node_modules", process.env.NODE_MODULES] },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        loader: "swc-loader",
        options: {
          jsc: {
            parser: { syntax: "typescript", tsx: true },
            target: "es2015",
            transform: { react: { runtime: "automatic", development: true } },
          },
        },
      },
      { test: /\.scss$/, use: ["style-loader", "css-loader", "sass-loader"] },
      { test: /\.png$/, type: "asset/resource" },
    ],
  },
  plugins: [
    new webpack.DefinePlugin({ GIT_HASH: JSON.stringify(env.COMMIT), BUILD_TIME: env.TIME }),
    new HtmlWebpackPlugin({ filename: "devtools.html", chunks: ["devtools"] }),
    new CopyWebpackPlugin({ patterns: [{ from: "data", to: "locales" }] }),
    new StorybookManifestPlugin({
      entry: "devtools",
      frame: path.join(__dirname, "src", "host", "Frame.tsx"),
      catalog: path.join(__dirname, "src", "host", "catalog.ts"),
      identity: { "<commit>": env.COMMIT, "<build time>": env.TIME },
      filename: "storybook-manifest.json",
    }),
  ],
});
