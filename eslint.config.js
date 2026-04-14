import js from "@eslint/js";
import tseslint from "typescript-eslint";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import jsxA11y from "eslint-plugin-jsx-a11y";
import importX from "eslint-plugin-import-x";
import security from "eslint-plugin-security";
import prettier from "eslint-config-prettier";
import globals from "globals";

export default tseslint.config(
  // Global ignores
  {
    ignores: [
      "dist/**",
      "src-tauri/**",
      "node_modules/**",
      "e2e/**",
      "*.config.{js,ts}",
      "wdio.conf.ts",
    ],
  },

  // Base JS rules
  js.configs.recommended,

  // TypeScript strict + stylistic
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },

  // React
  {
    ...react.configs.flat.recommended,
    settings: { react: { version: "detect" } },
  },
  react.configs.flat["jsx-runtime"],

  // React Hooks
  reactHooks.configs.flat.recommended,

  // Accessibility
  jsxA11y.flatConfigs.strict,

  // Import hygiene
  importX.flatConfigs.recommended,
  importX.flatConfigs.typescript,
  {
    settings: {
      "import-x/resolver": {
        typescript: {
          project: "./tsconfig.json",
        },
      },
    },
  },

  // Security (catches obvious anti-patterns)
  security.configs.recommended,

  // Project overrides
  {
    languageOptions: {
      globals: {
        ...globals.browser,
      },
    },
    rules: {
      // Enforce explicit return types on exported functions
      "@typescript-eslint/explicit-function-return-type": [
        "error",
        { allowExpressions: true, allowTypedFunctionExpressions: true },
      ],

      // Enforce consistent type imports
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { prefer: "type-imports", fixStyle: "inline-type-imports" },
      ],

      // No floating promises (critical for async correctness)
      "@typescript-eslint/no-floating-promises": "error",

      // No misused promises
      "@typescript-eslint/no-misused-promises": "error",

      // Require await in async functions
      "@typescript-eslint/require-await": "error",

      // Import ordering
      "import-x/order": [
        "error",
        {
          groups: [
            "builtin",
            "external",
            "internal",
            "parent",
            "sibling",
            "index",
            "type",
          ],
          "newlines-between": "always",
          alphabetize: { order: "asc" },
        },
      ],
      "import-x/no-duplicates": "error",

      // Security: the plugin is noisy on some rules in frontend code
      "security/detect-object-injection": "off",
    },
  },

  // Disable rules that conflict with prettier
  prettier,
);
