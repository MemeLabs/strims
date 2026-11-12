// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// prefer-arrow/prefer-arrow-functions as eslint-plugin-prefer-arrow 1.2.3
// has it with no options, as an oxlint JS plugin: that package's eslint
// peer (eslint 8, unsupported) was all it pulled in. A function
// declaration or expression is reported unless it is a constructor, a
// class method, a getter or setter, a generator, a function assigned
// into a prototype, or one whose own parameters or body use `this`.

const containsThis = (node) => {
  if (typeof node !== "object" || node === null) return false;
  if (Array.isArray(node)) return node.some(containsThis);
  if (node.type === "FunctionDeclaration" || node.type === "FunctionExpression") return false;
  if (node.type === "ThisExpression") return true;
  return Object.keys(node).some((field) => field !== "parent" && containsThis(node[field]));
};

const isPrototypeAssignment = (node) => {
  let parent = node.parent;
  while (parent) {
    switch (parent.type) {
      case "MemberExpression":
        if (parent.property && parent.property.name === "prototype") return true;
        parent = parent.object;
        break;
      case "AssignmentExpression":
        parent = parent.left;
        break;
      case "Property":
      case "ObjectExpression":
        parent = parent.parent;
        break;
      default:
        return false;
    }
  }
  return false;
};

const inspect = (context, node) => {
  const parent = node.parent;
  if (parent && parent.type === "MethodDefinition") return;
  if (containsThis(node.params) || containsThis(node.body)) return;
  if (node.generator) return;
  if (parent && (parent.kind === "get" || parent.kind === "set")) return;
  if (isPrototypeAssignment(node)) return;
  context.report({
    node,
    message:
      node.type === "FunctionDeclaration" && node.id && node.id.name
        ? "Use const or class constructors instead of named functions"
        : "Prefer using arrow functions over plain functions",
  });
};

export default {
  meta: { name: "prefer-arrow" },
  rules: {
    "prefer-arrow-functions": {
      meta: { docs: { description: "prefer arrow functions" }, schema: [] },
      create: (context) => ({
        "FunctionDeclaration:exit": (node) => inspect(context, node),
        "FunctionExpression:exit": (node) => inspect(context, node),
      }),
    },
  },
};
