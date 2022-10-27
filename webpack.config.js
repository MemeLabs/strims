const path = require("path");
const fs = require("fs");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const { CleanWebpackPlugin } = require("clean-webpack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const ReactRefreshTypeScript = require("react-refresh-typescript");
const ReactRefreshWebpackPlugin = require("@pmmmwh/react-refresh-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");
const WebpackPwaManifest = require("webpack-pwa-manifest");
const webpack = require("webpack");
const JSON5 = require("json5");
const { ChunkManifestPlugin } = require("./webpack/lib/ChunkManifestPlugin");

module.exports = (env, argv) => {
  const isProduction = argv.mode === "production";

  const scriptModuleRule = {
    test: /\.tsx?$/,
    exclude: /node_modules/,
    use: {
      loader: "ts-loader",
      options: {
        getCustomTransformers: () => ({
          before: [ReactRefreshTypeScript()],
        }),
        happyPackMode: true,
      },
    },
  };

  const styleModuleRule = {
    test: /\.s?css$/,
    use: [
      {
        loader: "css-loader",
        options: {
          url: true,
          sourceMap: true,
        },
      },
      {
        loader: "resolve-url-loader",
        options: {
          sourceMap: true,
        },
      },
      {
        loader: "postcss-loader",
        options: {
          sourceMap: true,
          postcssOptions: {
            plugins: ["autoprefixer"],
          },
        },
      },
      {
        loader: "sass-loader",
        options: {
          sourceMap: true,
        },
      },
    ],
  };

  const sharedRules = [
    scriptModuleRule,
    styleModuleRule,
    {
      test: /\.(png|jpg|gif|woff|woff2|eot|ttf|svg|mp4|m4s|m3u8)$/i,
      type: "asset/resource",
    },
    {
      test: /\.json$/i,
      include: [path.resolve(__dirname, "assets", "locales")],
      loader: "json5-loader",
      type: "javascript/auto",
    },
  ];

  const htmlWebpackPluginOptions = {
    meta: {
      viewport:
        "width=device-width, initial-scale=1, maximum-scale=1, viewport-fit=cover, shrink-to-fit=no",
      "apple-mobile-web-app-capable": "yes",
    },
  };

  const plugins = [
    new HtmlWebpackPlugin({
      chunks: ["index"],
      title: "Loading...",
      favicon: path.resolve(__dirname, "assets", "favicon.ico"),
      ...htmlWebpackPluginOptions,
    }),
    // new HtmlWebpackPlugin({
    //   filename: "funding.html",
    //   chunks: ["funding"],
    //   title: "funding",
    // }),
    new WebpackPwaManifest({
      name: "Strims",
      short_name: "Strims",
      description: "Live stream viewing with friends",
      background_color: "#222933",
      display: "fullscreen",
      // start_url: "/devtools.html",
      start_url: "/",
      orientation: "omit",
      ios: true,
      icons: [
        {
          src: path.resolve("assets/sggL.png"),
          sizes: [96, 128, 192, 256, 384, 512],
        },
      ],
    }),
    new CopyWebpackPlugin({
      patterns: [
        {
          from: path.resolve(__dirname, "assets", "locales"),
          to: "locales",
          transform: (content) => JSON.stringify(JSON5.parse(content)),
        },
        {
          from: path.resolve(__dirname, "node_modules", "emojibase-data"),
          to: "emoji",
          filter: (path) =>
            path.match(/\/[a-z]{2}(?:-[a-z]{2})?\/.*(compact|messages|cldr)\.json$/i),
        },
      ],
    }),
    new webpack.ProvidePlugin({
      process: "process/browser",
      Buffer: ["buffer", "Buffer"],
    }),
    new webpack.DefinePlugin({
      IS_PRODUCTION: isProduction,
      BUILD_TIME: Date.now(),
      GIT_HASH: JSON.stringify(
        require("child_process").execSync("git rev-parse HEAD").toString().trim()
      ),
      VERSION: JSON.stringify(require(path.resolve(__dirname, "package.json")).version),
      I18N_LANG: JSON.stringify(fs.readdirSync(path.resolve(__dirname, "assets", "locales"))),
      EMOJI_LANG: JSON.stringify(
        fs
          .readdirSync(path.resolve(__dirname, "node_modules", "emojibase-data"))
          .filter((path) => path.match(/^[a-z]{2}(?:-[a-z]{2,})?$/i))
      ),
    }),
  ];

  let devtool, optimization, entries;

  if (isProduction) {
    plugins.unshift(new CleanWebpackPlugin());

    scriptModuleRule.use = ["ts-loader"];

    styleModuleRule.use.unshift(MiniCssExtractPlugin.loader);

    plugins.push(
      new MiniCssExtractPlugin({
        filename: "[name].[contenthash].css",
        chunkFilename: "[id].[contenthash].css",
      }),
      new ChunkManifestPlugin(HtmlWebpackPlugin, {
        chunk: "index",
        varName: "MANIFEST",
        htmlFilename: "index.html",
      })
    );

    optimization = {
      minimizer: [
        new TerserPlugin({
          parallel: true,
        }),
      ],
    };

    devtool = "source-map";
  } else {
    plugins.unshift(
      new ReactRefreshWebpackPlugin({
        exclude: /node_modules|\.(shared-)?worker\.ts|\/src\/web\/index\.tsx|src\/lib\/i18n\.ts$/,
        overlay: false,
      }),
      new HtmlWebpackPlugin({
        filename: "test.html",
        chunks: ["test"],
        title: "test",
      }),
      new HtmlWebpackPlugin({
        filename: "devtools.html",
        chunks: ["devtools"],
        title: "devtools",
        ...htmlWebpackPluginOptions,
      }),
      new webpack.DefinePlugin({
        MANIFEST: "[]",
      })
    );

    entries = {
      test: path.join(__dirname, "src", "web", "test.ts"),
      devtools: path.join(__dirname, "src", "devtools", "index.tsx"),
    };

    styleModuleRule.use.unshift("style-loader");

    devtool = "eval-source-map";
  }

  const createElectronBuild = (target, fileName) => ({
    target: `electron-${target}`,
    entry: {
      [target]: path.join(__dirname, "src", "desktop", fileName || `${target}.ts`),
    },
    devtool,
    output: {
      filename: "[name].js",
      chunkFilename: "[id].js",
      path: path.resolve(__dirname, "dist", "desktop"),
    },
    module: {
      rules: sharedRules,
    },
    resolve: {
      extensions: [".tsx", ".ts", ".js"],
    },
    node: {
      __dirname: false,
      __filename: false,
    },
    optimization,
  });

  const tlsCertRoot = path.join(__dirname, "hack", "tls");
  const devServerOptions = fs.existsSync(path.join(tlsCertRoot, "development.key"))
    ? {
        key: fs.readFileSync(path.join(tlsCertRoot, "development.key")),
        cert: fs.readFileSync(path.join(tlsCertRoot, "development.crt")),
        ca: fs.readFileSync(path.join(tlsCertRoot, "development-ca.crt")),
      }
    : {};

  switch (env.TARGET) {
    case "app":
      return [
        Object.assign(createElectronBuild("renderer", "renderer.tsx"), {
          plugins: [
            new HtmlWebpackPlugin({
              title: "Loading...",
            }),
          ],
        }),
        createElectronBuild("main"),
        createElectronBuild("preload"),
      ];
    case "web":
    default:
      return [
        {
          target: "web",
          stats: {
            children: true,
            errorDetails: true,
          },
          entry: {
            index: path.join(__dirname, "src", "web", "index.tsx"),
            sw: path.join(__dirname, "src", "web", "sw.ts"),
            // funding: path.join(__dirname, "src", "funding", "index.tsx"),
            ...entries,
          },
          devtool,
          output: {
            filename: "[name].[contenthash].js",
            chunkFilename: "[id].[chunkhash].js",
            path: path.resolve(__dirname, "dist", "web"),
            publicPath: "/",
          },
          devServer: {
            server: {
              type: "https",
              options: devServerOptions,
            },
            hot: "only",
            compress: false,
            historyApiFallback: {
              index: "/",
            },
            host: "0.0.0.0",
            proxy: {
              "/manage": {
                target: "ws://localhost:8083",
                ws: true,
              },
              "/api/funding": {
                target: "ws://localhost:8084",
                ws: true,
              },
              "/api/invite": {
                target: "http://localhost:8084",
              },
            },
          },
          module: {
            rules: [
              {
                test: /\.worker\.ts$/,
                loader: "worker-loader",
                options: {
                  inline: "fallback",
                  chunkFilename: "[id].[contenthash].worker.js",
                },
              },
              {
                test: /\.shared-worker\.ts$/,
                loader: "worker-loader",
                options: {
                  inline: "fallback",
                  worker: "SharedWorker",
                  chunkFilename: "[id].[contenthash].shared-worker.js",
                },
              },
              {
                test: /\.go/,
                use: ["golang-wasm-async-loader"],
              },
              {
                test: /\.wasm$/,
                loader: "file-loader",
              },
              ...sharedRules,
            ],
          },
          resolve: {
            extensions: [".go", ".tsx", ".ts", ".js"],
            fallback: {
              "fs": false,
              "stream": require.resolve("stream-browserify"),
              "buffer": require.resolve("buffer"),
              "process": require.resolve("process"),
            },
          },
          optimization,
          plugins,
        },
      ];
  }
};
