const path = require("path");
const fs = require("fs");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");
const ReactRefreshWebpackPlugin = require("@pmmmwh/react-refresh-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");
const webpack = require("webpack");
const JSON5 = require("json5");
const { ChunkManifestPlugin } = require("./webpack/lib/ChunkManifestPlugin");
const { PwaManifestPlugin } = require("./webpack/lib/PwaManifestPlugin");

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
            runtime: "automatic",
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

  // A worker imports a wasm crate's Cargo.toml (crates/strims-wasm, the node;
  // crates/strims-player, the player), which the loader turns into the
  // crate's wasm-bindgen glue.
  const wasmCrateRule = (crate, features) => ({
    test: new RegExp(`crates/${crate}/Cargo\\.toml$`),
    type: "javascript/auto",
    loader: path.resolve(__dirname, "vendor_modules", "strims-wasm-loader", "index.js"),
    options: {
      profile: isProduction ? "release-wasm" : "dev",
      features,
    },
  });

  // The glue fetches its module from new URL("<lib>_bg.wasm", import.meta.url).
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
          sassOptions: {
            // What Dart Sass 3 removes and the styles still use: @import and
            // the global functions (mixins.scss's map-get and the like).
            silenceDeprecations: ["import", "global-builtin"],
          },
        },
      },
    ],
  };

  const sharedRules = [
    scriptModuleRule,
    styleModuleRule,
    {
      test: /\.(png|jpg|gif|woff|woff2|eot|ttf|svg)$/i,
      type: "asset/resource",
    },
    {
      test: /\.json$/i,
      include: [path.resolve(__dirname, "assets", "locales")],
      // The locales are JSON5 (comments, trailing commas).
      type: "json",
      parser: { parse: JSON5.parse },
    },
  ];

  const htmlWebpackPluginOptions = {
    meta: {
      viewport:
        "width=device-width, initial-scale=1, maximum-scale=1, viewport-fit=cover, shrink-to-fit=no",
      "apple-mobile-web-app-capable": "yes",
    },
  };

  // sass's browser build (the chat style sheet form's compiler) keeps the
  // bundle's require as a global of its own (self.require = require), for
  // its Node code paths.
  const ignoreWarnings = [
    { module: /node_modules\/sass\/sass\.dart\.js$/, message: /^Critical dependency/ },
  ];

  // The emojibase-data locales the chat provider can ask for (EMOJI_LANG) and
  // the files of each it reads, both matched within the package: an absolute
  // path's own directories (a checkout under /ui) could match the locale part.
  const emojiDataDir = path.resolve(__dirname, "node_modules", "emojibase-data");
  const emojiLang = /^[a-z]{2}(?:-[a-z]{2,})?$/i;
  const emojiDataCopy = {
    from: emojiDataDir,
    to: "emoji",
    filter: (file) => {
      const [lang, ...rest] = path.relative(emojiDataDir, file).split(path.sep);
      return (
        emojiLang.test(lang) && /^(?:compact|messages|shortcodes\/cldr)\.json$/.test(rest.join("/"))
      );
    },
  };

  const providePlugin = () =>
    new webpack.ProvidePlugin({
      // With the extension: an ES module's requests are fully specified (vega's are).
      process: "process/browser.js",
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
      // The origin of the native node whose bootstrap API the watch page
      // reads (WATCH_API_URL=https://host:port); unset, the page's own.
      WATCH_API_URL: JSON.stringify(process.env.WATCH_API_URL || ""),
      EMOJI_LANG: JSON.stringify(fs.readdirSync(emojiDataDir).filter((dir) => emojiLang.test(dir))),
    });

  const plugins = [
    new HtmlWebpackPlugin({
      chunks: ["index"],
      title: "Loading...",
      favicon: path.resolve(__dirname, "assets", "favicon.ico"),
      ...htmlWebpackPluginOptions,
    }),
    // The one-click watch page, served at /watch: tiles and a player, no
    // branding, so no favicon, title or app manifest of its own.
    new HtmlWebpackPlugin({
      filename: "watch.html",
      chunks: ["watch"],
      title: "",
      ...htmlWebpackPluginOptions,
    }),
    new PwaManifestPlugin(HtmlWebpackPlugin, {
      manifest: {
        name: "Strims",
        short_name: "Strims",
        display: "fullscreen",
        start_url: "/",
        description: "Live stream viewing with friends",
        background_color: "#222933",
      },
      // assets/sggL.png at each size, largest first.
      icons: [512, 384, 256, 192, 128, 96].map((size) => ({
        src: path.resolve(__dirname, "assets", "icons", `icon_${size}x${size}.png`),
        size,
      })),
      exclude: ["watch.html"],
    }),
    new CopyWebpackPlugin({
      patterns: [
        {
          from: path.resolve(__dirname, "assets", "locales"),
          to: "locales",
          transform: (content) => JSON.stringify(JSON5.parse(content)),
        },
        emojiDataCopy,
      ],
    }),
    providePlugin(),
    definePlugin(),
  ];

  let devtool, optimization, entries;

  if (isProduction) {
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
          /node_modules|\/target\/|\.(shared-)?worker\.ts|\/src\/web\/sw\.ts$|\/src\/(web|watch)\/index\.tsx|src\/lib\/i18n\.ts$/,
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
            rules: [
              wasmCrateRule("strims-wasm", ["e2e"]),
              wasmCrateRule("strims-player", []),
              wasmAssetRule,
              scriptRule(false),
            ],
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
    // The testbed's browser nodes (hack/testbed): e2e.html alone, in its own
    // dist dir, so a production build (release wasm) leaves the tests' alone.
    case "wasm-testbed":
      return [
        {
          target: "web",
          entry: {
            e2e: path.join(__dirname, "src", "web", "e2e.ts"),
          },
          devtool,
          output: {
            filename: "[name].[contenthash].js",
            chunkFilename: "[id].[chunkhash].js",
            path: path.resolve(__dirname, "dist", "wasm-testbed"),
            publicPath: "/",
            clean: true,
          },
          module: {
            rules: [
              wasmCrateRule("strims-wasm", ["e2e"]),
              wasmCrateRule("strims-player", []),
              wasmAssetRule,
              scriptRule(false),
            ],
          },
          resolve,
          optimization,
          plugins: [
            new HtmlWebpackPlugin({ filename: "e2e.html", chunks: ["e2e"], title: "e2e" }),
            providePlugin(),
            definePlugin(),
          ],
        },
      ];
    // The sim-results console's page alone, which hack/console/server.py
    // serves: the devtools app with the wasm crates' glue stubbed
    // (src/devtools/wasm-stub.ts), so it builds without Rust, into
    // target/console/dist.
    case "console":
      return [
        {
          target: "web",
          entry: {
            devtools: path.join(__dirname, "src", "devtools", "index.tsx"),
          },
          devtool: "source-map",
          output: {
            filename: "[name].[contenthash].js",
            chunkFilename: "[id].[contenthash].js",
            path: path.resolve(__dirname, "target", "console", "dist"),
            publicPath: "/",
            clean: true,
          },
          // A file's name holds its bytes' hash, which server.py caches for
          // good by (a development build's hashes are its modules' alone).
          optimization: {
            realContentHash: true,
          },
          cache: {
            type: "filesystem",
            cacheDirectory: path.resolve(__dirname, "target", "console", "webpack-cache"),
          },
          // The machine's inotify watchers run out on the whole tree.
          watchOptions: {
            ignored: [
              "**/node_modules/**",
              "**/target/**",
              "**/crates/**",
              "**/dist/**",
              "**/.git/**",
            ],
          },
          module: {
            // No React Refresh: nothing serves hot updates.
            rules: [scriptRule(false), ...sharedRules.filter((r) => r !== scriptModuleRule)],
          },
          resolve,
          ignoreWarnings,
          plugins: [
            new webpack.NormalModuleReplacementPlugin(
              /crates\/[^/]+\/Cargo\.toml$/,
              path.resolve(__dirname, "src", "devtools", "wasm-stub.ts")
            ),
            new HtmlWebpackPlugin({
              filename: "devtools.html",
              chunks: ["devtools"],
              title: "devtools",
              ...htmlWebpackPluginOptions,
            }),
            new CopyWebpackPlugin({
              patterns: [
                {
                  from: path.resolve(__dirname, "assets", "locales"),
                  to: "locales",
                  transform: (content) => JSON.stringify(JSON5.parse(content)),
                },
                // The chat provider's emoji data, as the web target has it.
                emojiDataCopy,
              ],
            }),
            new webpack.DefinePlugin({
              MANIFEST: "[]",
            }),
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
            watch: path.join(__dirname, "src", "watch", "index.tsx"),
            ...entries,
          },
          devtool,
          output: {
            filename: "[name].[contenthash].js",
            chunkFilename: "[id].[chunkhash].js",
            path: path.resolve(__dirname, "dist", "web"),
            publicPath: "/",
            clean: isProduction,
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
              rewrites: [{ from: /^\/watch(\/|$)/, to: "/watch.html" }],
            },
            host: "0.0.0.0",
            proxy: [
              {
                context: "/manage",
                target: "ws://localhost:8083",
                ws: true,
              },
              {
                // A local native node's bootstrap API, on its HLS listener
                // (--hls-listen 127.0.0.1:8084).
                context: "/api/v1",
                target: "http://localhost:8084",
                // The browser's response closing ends the upstream request
                // (httpxy, http-proxy-middleware's proxy, does so too; this
                // doesn't depend on it): a closed event stream would hold its
                // upstream connection, and one of the API's stream slots.
                on: {
                  proxyReq: (proxyReq, req, res) => {
                    res.on("close", () => {
                      if (!res.writableFinished) proxyReq.destroy();
                    });
                  },
                },
              },
              {
                // The sim-results console's API (hack/console/server.py), which
                // answers only requests whose Host names this machine.
                context: "/console",
                target: "http://localhost:8086",
                changeOrigin: true,
                // As /api/v1's: a fetch's streamed output ends with the browser's response.
                on: {
                  proxyReq: (proxyReq, req, res) => {
                    res.on("close", () => {
                      if (!res.writableFinished) proxyReq.destroy();
                    });
                  },
                },
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
            rules: [
              wasmCrateRule("strims-wasm", []),
              wasmCrateRule("strims-player", []),
              wasmAssetRule,
              ...sharedRules,
            ],
          },
          resolve,
          ignoreWarnings,
          optimization,
          plugins,
        },
      ];
  }
};
