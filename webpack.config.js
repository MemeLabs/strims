const path = require("path");
const fs = require("fs");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const { CleanWebpackPlugin } = require("clean-webpack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const ReactRefreshTypeScript = require("react-refresh-typescript");
const ReactRefreshWebpackPlugin = require("@pmmmwh/react-refresh-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");
const WebpackPwaManifest = require("webpack-pwa-manifest");
const webpack = require("webpack");

module.exports = (env, argv) => {
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

  const staticModuleRule = {
    test: /\.(png|jpg|gif|woff|woff2|eot|ttf|svg)$/i,
    type: "asset/resource",
  };

  const htmlWebpackPluginOptions = {
    meta: {
      viewport: "width=device-width, initial-scale=1, viewport-fit=cover, shrink-to-fit=no",
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
      start_url: "/devtools.html",
      orientation: "omit",
      ios: true,
      icons: [
        {
          src: path.resolve("assets/splat.png"),
          sizes: [96, 128, 192, 256, 384, 512], // multiple sizes
        },
      ],
    }),
    new webpack.ProvidePlugin({
      process: "process/browser",
      Buffer: ["buffer", "Buffer"],
    }),
  ];

  let devtool, optimization;

  if (argv.mode === "production") {
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
        exclude: /node_modules|\.(shared-)?worker\.ts|\/src\/web\/index\.tsx$/,
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
      rules: [scriptModuleRule, styleModuleRule, staticModuleRule],
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
              scriptModuleRule,
              styleModuleRule,
              staticModuleRule,
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
