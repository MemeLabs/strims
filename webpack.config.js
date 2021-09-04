const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const { CleanWebpackPlugin } = require("clean-webpack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const TerserPlugin = require("terser-webpack-plugin");
const webpack = require("webpack");

module.exports = (env, argv) => {
  const scriptModuleRule = {
    test: /\.tsx?$/,
    use: [
      {
        loader: "ts-loader",
        options: {
          transpileOnly: true,
        },
      },
    ],
    exclude: /node_modules/,
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

  const plugins = [
    new HtmlWebpackPlugin({
      chunks: ["index"],
      title: "Loading...",
      favicon: path.resolve(__dirname, "assets", "favicon.ico"),
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
    }),
    // new HtmlWebpackPlugin({
    //   filename: "funding.html",
    //   chunks: ["funding"],
    //   title: "funding",
    // }),
    new webpack.HotModuleReplacementPlugin(),
    new webpack.ProvidePlugin({
      process: "process/browser",
      Buffer: ["buffer", "Buffer"],
    }),
  ];

  let devtool;

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

    devtool = "source-map";
  } else {
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
    optimization: {
      minimizer: [
        new TerserPlugin({
          parallel: true,
        }),
      ],
    },
  });

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
            https: true,
            hot: true,
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
          optimization: {
            minimizer: [
              new TerserPlugin({
                parallel: true,
              }),
            ],
          },
          plugins,
        },
      ];
  }
};
