const path = require("path");
const fs = require("fs");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const { CleanWebpackPlugin } = require("clean-webpack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const ReactRefreshWebpackPlugin = require("@pmmmwh/react-refresh-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");
const WebpackPwaManifest = require("webpack-pwa-manifest");
const webpack = require("webpack");
const JSON5 = require("json5");
const { ChunkManifestPlugin } = require("./webpack/lib/ChunkManifestPlugin");

module.exports = (env, argv) => {
  const isProduction = argv.mode === "production";

  // swc only strips types; type checking is done separately by tsc
  const swcLoader = (tsx, refresh) => ({
    loader: "swc-loader",
    options: {
      jsc: {
        parser: { syntax: "typescript", tsx },
        target: "es2015",
        transform: {
          useDefineForClassFields: false,
          react: {
            runtime: "classic",
            development: !isProduction,
            refresh,
          },
        },
      },
    },
  });

  // refresh: whether ReactRefreshWebpackPlugin runs, which defines the hooks
  // swc's refresh transform calls
  const scriptRule = (refresh) => ({
    test: /\.tsx?$/,
    exclude: /node_modules/,
    oneOf: [{ test: /\.tsx$/, use: swcLoader(true, refresh) }, { use: swcLoader(false, refresh) }],
  });

  const scriptModuleRule = scriptRule(!isProduction);

  // The worker imports crates/strims-wasm/Cargo.toml, which the loader turns
  // into the crate's wasm-bindgen glue.
  const strimsWasmRule = (features) => ({
    test: /crates\/strims-wasm\/Cargo\.toml$/,
    type: "javascript/auto",
    loader: path.resolve(__dirname, "vendor_modules", "strims-wasm-loader", "index.js"),
    options: {
      profile: isProduction ? "release-wasm" : "dev",
      features,
    },
  });

  // The glue fetches its module from new URL("strims_wasm_bg.wasm", import.meta.url).
  const wasmAssetRule = {
    test: /\.wasm$/,
    type: "asset/resource",
    generator: {
      filename: "[name].[contenthash][ext]",
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

  const providePlugin = () =>
    new webpack.ProvidePlugin({
      process: "process/browser",
      Buffer: ["buffer", "Buffer"],
    });

  const definePlugin = () =>
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
    });

  const plugins = [
    new HtmlWebpackPlugin({
      chunks: ["index"],
      title: "Loading...",
      favicon: path.resolve(__dirname, "assets", "favicon.ico"),
      ...htmlWebpackPluginOptions,
    }),
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
    providePlugin(),
    definePlugin(),
  ];

  let devtool, optimization, entries;

  if (isProduction) {
    plugins.unshift(new CleanWebpackPlugin());

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
        exclude:
          /node_modules|\/target\/|\.(shared-)?worker\.ts|\/src\/web\/index\.tsx|src\/lib\/i18n\.ts$/,
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

  const resolve = {
    extensions: [".tsx", ".ts", ".js"],
    fallback: {
      "fs": false,
      "stream": require.resolve("stream-browserify"),
      "buffer": require.resolve("buffer"),
      "process": require.resolve("process"),
    },
  };

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
    // The pages the strims-wasm browser tests load: test.html and e2e.html.
    // A static server serves dist/wasm-e2e at its root; asset URLs start at /.
    case "wasm-e2e":
      return [
        {
          target: "web",
          entry: {
            test: path.join(__dirname, "src", "web", "test.ts"),
            e2e: path.join(__dirname, "src", "web", "e2e.ts"),
          },
          devtool,
          output: {
            filename: "[name].[contenthash].js",
            chunkFilename: "[id].[chunkhash].js",
            path: path.resolve(__dirname, "dist", "wasm-e2e"),
            publicPath: "/",
            clean: true,
          },
          module: {
            rules: [strimsWasmRule(["e2e"]), wasmAssetRule, scriptRule(false)],
          },
          resolve,
          optimization,
          plugins: [
            new HtmlWebpackPlugin({ filename: "test.html", chunks: ["test"], title: "test" }),
            new HtmlWebpackPlugin({ filename: "e2e.html", chunks: ["e2e"], title: "e2e" }),
            providePlugin(),
            definePlugin(),
          ],
        },
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
            proxy: [
              {
                context: "/manage",
                target: "ws://localhost:8083",
                ws: true,
              },
              {
                context: "/api/invite",
                target: "http://localhost:8084",
              },
              {
                // A local native node's WS listener, which a node in the page
                // dials as wss://<LAN IP>:8080/<hex key>.
                context: (pathname) => /^\/[0-9a-f]{64}$/.test(pathname),
                target: "ws://localhost:8082",
                ws: true,
              },
            ],
          },
          module: {
            rules: [strimsWasmRule([]), wasmAssetRule, ...sharedRules],
          },
          resolve,
          optimization,
          plugins,
        },
      ];
  }
};
