module.exports = {
  env: {
    browser: true,
    node: true,
    worker: true,
    es2020: true,
  },
  extends: [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
    "plugin:@typescript-eslint/recommended-requiring-type-checking",
    "plugin:jsdoc/recommended",
    "prettier",
  ],
  parser: "@typescript-eslint/parser",
  parserOptions: {
    project: "./tsconfig.json",
    sourceType: "module",
  },
  plugins: ["@typescript-eslint", "prefer-arrow"],
  rules: {
    "@typescript-eslint/interface-name-prefix": "off",
    "@typescript-eslint/no-inferrable-types": "off",
    "@typescript-eslint/no-namespace": "off",
    "@typescript-eslint/no-explicit-any": "off",
    "@typescript-eslint/ban-ts-comment": "off",
    "@typescript-eslint/no-misused-promises": "off",
    "prefer-arrow/prefer-arrow-functions": "error",
    "max-classes-per-file": "off",
    "max-len": [
      "warn",
      {
        "code": 100,
        "ignoreComments": true,
        "ignoreStrings": true,
        "ignoreTemplateLiterals": true,
      },
    ],
    "no-bitwise": "off",
    "no-console": "warn",
    "quote-props": ["error", "consistent"],
  },
};
