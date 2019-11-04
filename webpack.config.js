const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CleanWebpackPlugin = require('clean-webpack-plugin');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const TerserPlugin = require('terser-webpack-plugin');
const webpack = require('webpack');

module.exports = (env, argv) => {
  const scriptModuleRule = {
    test: /\.tsx?$/,
    use: [
      'ts-loader',
    ],
    exclude: /node_modules/
  };

  const styleModuleRule = {
    test: /\.s?css$/,
    use: [
      'css-loader',
      'resolve-url-loader',
      {
        loader: 'postcss-loader',
        options: {
          ident: 'postcss',
          plugins: () => [
            require('autoprefixer')({
              browsers: ['last 2 versions'],
            })
          ]
        }
      },
      'sass-loader',
    ]
  };

  const plugins = [
    new CleanWebpackPlugin(),
    new HtmlWebpackPlugin({
      title: 'Loading...',
      favicon: path.resolve(__dirname, 'assets', 'favicon.ico'),
    }),
    new webpack.HotModuleReplacementPlugin(),
  ];

  if (argv.mode === 'production') {
    styleModuleRule.use.unshift(MiniCssExtractPlugin.loader);

    plugins.push(new MiniCssExtractPlugin({
      filename: '[name].[contenthash].css',
      chunkFilename: '[id].[contenthash].css'
    }));
  } else {
    styleModuleRule.use.unshift('style-loader');
  }

  return {
    entry: {
      app: './src/index.tsx',
    },
    devtool: 'inline-source-map',
    output: {
      filename: '[name].[hash].js',
      chunkFilename: '[id].[chunkhash].js',
      path: path.resolve(__dirname, 'dist'),
      publicPath: '/',
    },
    devServer: {
      // https: true,
      hot: false,
      historyApiFallback: {
        index: '/'
      },
      host: '0.0.0.0',
      proxy: {
        // '/api': 'http://localhost:8081',
      },
      contentBase: [
        path.join(__dirname, 'pkg'),
        path.join(__dirname, 'assets')
      ],
    },
    watch: true,
    module: {
      rules: [
        scriptModuleRule,
        styleModuleRule,
        {
          test: /\.go/,
          use: ['golang-wasm-async-loader'],
        },
        {
          test: /\.(png|jpg|gif|woff|woff2|eot|ttf|svg)$/i,
          use: [
            {
              loader: 'url-loader',
              options: {
                limit: 8192,
              },
            },
          ],
        },
        {
          test: /\.(wasm)$/,
          loader: "file",
        }
      ]
    },
    node: {
        fs: false,
    },
    resolve: {
      extensions: [ '.go', '.tsx', '.ts', '.js' ]
    },
    optimization: {
      minimizer: [
        new TerserPlugin({
          cache: true,
          parallel: true,
        }),
      ],
    },
    plugins
  };
}
