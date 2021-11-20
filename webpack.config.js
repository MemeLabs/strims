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
      test: /\.(png|jpg|gif|woff|woff2|eot|ttf|svg|mp4|m4s)$/i,
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
      "theme:light": {
        name: "theme-color",
        content: "#d5d5d5",
        media: "(prefers-color-scheme: light)",
      },
      "theme:dark": {
        name: "theme-color",
        content: "#222933",
        media: "(prefers-color-scheme: dark)",
      },
      "apple-mobile-web-app-capable": "yes",
      "apple-mobile-web-app-status-bar-style": "black",
    },
  };

  const plugins = [
    new HtmlWebpackPlugin({
      chunks: ["index"],
      title: "Loading...",
      favicon: path.resolve(__dirname, "assets", "favicon.ico"),
      ...htmlWebpackPluginOptions,
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
          src: path.resolve("assets/splat.png"),
          sizes: [96, 128, 192, 256, 384, 512], // multiple sizes
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
    }),
  ];

  let devtool, optimization;

  if (isProduction) {
    plugins.unshift(new CleanWebpackPlugin());

    scriptModuleRule.use = ["ts-loader"];

    styleModuleRule.use.unshift(MiniCssExtractPlugin.loader);

    plugins.push(
      new MiniCssExtractPlugin({
        filename: "[name].[contenthash].css",
        chunkFilename: "[id].[contenthash].css",
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
      })
    );

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
  const devServerHttps = fs.existsSync(tlsCertRoot)
    ? {
        key: fs.readFileSync(path.join(tlsCertRoot, "development.key")),
        cert: fs.readFileSync(path.join(tlsCertRoot, "development.crt")),
        cacert: fs.readFileSync(path.join(tlsCertRoot, "development-ca.crt")),
      }
    : true;

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
          entry: {
            index: path.join(__dirname, "src", "web", "index.tsx"),
            test: path.join(__dirname, "src", "web", "test.ts"),
            devtools: path.join(__dirname, "src", "devtools", "index.tsx"),
            sw: path.join(__dirname, "src", "devtools", "sw.ts"),
            // funding: path.join(__dirname, "src", "funding", "index.tsx"),
          },
          devtool,
          output: {
            filename: "[name].[contenthash].js",
            chunkFilename: "[id].[chunkhash].js",
            path: path.resolve(__dirname, "dist", "web"),
            publicPath: "/",
          },
          devServer: {
            https: devServerHttps,
            hot: true,
            compress: false,
            historyApiFallback: {
              index: "/",
            },
            host: "0.0.0.0",
            proxy: {
              "/test-bootstrap": {
                target: "ws://localhost:8082",
                ws: true,
              },
              "/manage": {
                target: "ws://localhost:8083",
                ws: true,
              },
              "/api": {
                target: "ws://localhost:8084",
                ws: true,
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
