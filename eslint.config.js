// For more info, see https://github.com/storybookjs/eslint-plugin-storybook#configuration-flat-config-format
import storybook from 'eslint-plugin-storybook';

import js from '@eslint/js';
import eslintConfigPrettier from 'eslint-config-prettier';
import jsxA11y from 'eslint-plugin-jsx-a11y';
import promise from 'eslint-plugin-promise';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import regexp from 'eslint-plugin-regexp';
import simpleImportSort from 'eslint-plugin-simple-import-sort';
import unicorn from 'eslint-plugin-unicorn';
import globals from 'globals';
import tseslint from 'typescript-eslint';

const visibleTextPattern = /\p{Letter}/u;
const translatedAttributeNames = new Set([
  'aria-label',
  'cancelText',
  'copyLabel',
  'description',
  'label',
  'notFoundContent',
  'okText',
  'placeholder',
  'repeatLabel',
  'title',
]);
const translatedObjectPropertyNames = new Set(['description', 'label', 'title']);
const technicalAttributeNames = new Set([
  'accept',
  'buttonStyle',
  'className',
  'format',
  'key',
  'mode',
  'name',
  'picker',
  'role',
  'rowKey',
  'size',
  'to',
  'type',
  'value',
  'variant',
]);
const technicalStringPatterns = [
  /^[#.]/,
  /^--/,
  /^\//,
  /^https?:\/\//,
  /^[\w./:-]+$/,
  /^[A-Z][\w-]+:\s/,
];

const hasVisibleText = (value) => visibleTextPattern.test(value);

const isTechnicalString = (value) =>
  technicalStringPatterns.some((pattern) => pattern.test(value.trim()));

const getStaticString = (node) => {
  if (!node) {
    return false;
  }

  if (node.type === 'Literal' && typeof node.value === 'string') {
    return node.value;
  }

  if (node.type === 'TemplateLiteral' && node.expressions.length === 0) {
    return node.quasis.map((quasi) => quasi.value.cooked ?? '').join('');
  }

  return false;
};

const getPropertyName = (node) => {
  if (node.type === 'Identifier') {
    return node.name;
  }

  if (node.type === 'Literal' && typeof node.value === 'string') {
    return node.value;
  }

  return false;
};

const localI18nPlugin = {
  rules: {
    'no-untranslated-text': {
      create(context) {
        const report = (node, value) => {
          const normalizedValue = value.trim();

          if (!hasVisibleText(normalizedValue) || isTechnicalString(normalizedValue)) {
            return;
          }

          context.report({
            data: {
              value: normalizedValue,
            },
            message: 'Move visible UI text to i18n resources: "{{value}}".',
            node,
          });
        };

        return {
          CallExpression(node) {
            if (
              node.callee.type !== 'MemberExpression' ||
              node.callee.property.type !== 'Identifier' ||
              !['error', 'info', 'success', 'warning'].includes(node.callee.property.name)
            ) {
              return;
            }

            const value = getStaticString(node.arguments[0]);

            if (value !== false) {
              report(node.arguments[0], value);
            }
          },
          JSXAttribute(node) {
            if (node.name.type !== 'JSXIdentifier') {
              return;
            }

            const attributeName = node.name.name;

            if (technicalAttributeNames.has(attributeName)) {
              return;
            }

            if (!translatedAttributeNames.has(attributeName)) {
              return;
            }

            if (node.value?.type === 'Literal' && typeof node.value.value === 'string') {
              report(node.value, node.value.value);
              return;
            }

            if (node.value?.type !== 'JSXExpressionContainer') {
              return;
            }

            const value = getStaticString(node.value.expression);

            if (value !== false) {
              report(node.value.expression, value);
            }
          },
          JSXExpressionContainer(node) {
            if (node.parent.type === 'JSXAttribute') {
              return;
            }

            const value = getStaticString(node.expression);

            if (value !== false) {
              report(node.expression, value);
            }
          },
          JSXText(node) {
            report(node, node.value);
          },
          Property(node) {
            const propertyName = getPropertyName(node.key);

            if (propertyName === false || !translatedObjectPropertyNames.has(propertyName)) {
              return;
            }

            const value = getStaticString(node.value);

            if (value !== false) {
              report(node.value, value);
            }
          },
        };
      },
      meta: {
        docs: {
          description: 'Disallow visible UI text outside i18n resources.',
        },
        messages: {},
        schema: [],
        type: 'problem',
      },
    },
  },
};

export default tseslint.config(
  {
    ignores: [
      'dist',
      'storybook-static',
      'src-tauri/gen',
      'src-tauri/target',
      'src-tauri/extensions',
      'coverage',
      'node_modules',
      '.venv',
      '.codex',
      'ui-audit-artifacts',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked.map((config) => ({
    ...config,
    files: ['**/*.{ts,tsx}'],
  })),
  ...tseslint.configs.stylisticTypeChecked.map((config) => ({
    ...config,
    files: ['**/*.{ts,tsx}'],
  })),
  jsxA11y.flatConfigs.recommended,
  unicorn.configs['flat/recommended'],
  promise.configs['flat/recommended'],
  regexp.configs['flat/recommended'],
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.es2024,
      },
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: {
      'local-i18n': localI18nPlugin,
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
      'simple-import-sort': simpleImportSort,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      '@typescript-eslint/consistent-type-imports': [
        'error',
        {
          fixStyle: 'inline-type-imports',
          prefer: 'type-imports',
        },
      ],
      '@typescript-eslint/no-empty-function': 'off',
      '@typescript-eslint/no-magic-numbers': 'off',
      '@typescript-eslint/no-unnecessary-condition': 'warn',
      '@typescript-eslint/restrict-template-expressions': [
        'error',
        {
          allowBoolean: true,
          allowNever: true,
          allowNullish: true,
          allowNumber: true,
        },
      ],
      'import/order': 'off',
      'react-refresh/only-export-components': [
        'warn',
        {
          allowConstantExport: true,
        },
      ],
      'simple-import-sort/exports': 'error',
      'simple-import-sort/imports': [
        'error',
        {
          groups: [
            ['^react$', '^react-dom', '^react-router', String.raw`^@?\w`],
            ['^#/(app|pages|components|ui|shared|styles)(/.*|$)'],
            [String.raw`^\u0000`],
            [String.raw`^\.\.(?!/?$)`, String.raw`^\.\./?$`],
            [String.raw`^\./(?=.*/)(?!/?$)`, String.raw`^\.(?!/?$)`, String.raw`^\./?$`],
            [String.raw`^.+\.s?css$`],
          ],
        },
      ],
      'local-i18n/no-untranslated-text': 'error',
      'unicorn/filename-case': [
        'error',
        {
          cases: {
            camelCase: true,
            pascalCase: true,
          },
          ignore: ['vite-env.d.ts'],
        },
      ],
      'unicorn/no-null': 'off',
      'unicorn/prevent-abbreviations': 'off',
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
  },
  {
    files: ['src/app/I18nProvider/resources.ts', 'src/mocks/**/*.ts'],
    rules: {
      'local-i18n/no-untranslated-text': 'off',
    },
  },
  {
    files: ['*.config.{js,ts}', 'eslint.config.js'],
    rules: {
      'unicorn/prefer-module': 'off',
    },
  },
  {
    files: ['scripts/**/*.mjs'],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
    rules: {
      'unicorn/no-process-exit': 'off',
      'unicorn/no-null': 'off',
      'unicorn/prevent-abbreviations': 'off',
    },
  },
  {
    files: ['*.cjs'],
    rules: {
      'unicorn/no-null': 'off',
    },
  },
  {
    files: ['**/index.ts'],
    rules: {
      'unicorn/prefer-export-from': 'off',
    },
  },
  eslintConfigPrettier,
  storybook.configs['flat/recommended'],
);
