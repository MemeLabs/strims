module.exports = {
  env: {
    browser: true,
    node: true,
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
    "prefer-arrow/prefer-arrow-functions": "error",
    "complexity": "off",
    "max-classes-per-file": "off",
    "max-len": "off",
    "no-bitwise": "off",
    "no-console": "warn",
    "valid-typeof": "off",
    "quote-props": ["error", "consistent"],
    // TODO: remove after this is fixed https://github.com/react-hook-form/react-hook-form/issues/2887
    "@typescript-eslint/unbound-method": "off",
  },
};
