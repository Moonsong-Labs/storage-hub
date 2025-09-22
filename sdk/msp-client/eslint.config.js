import tsParser from '@typescript-eslint/parser';
import tsPlugin from '@typescript-eslint/eslint-plugin';
import simpleImportSort from 'eslint-plugin-simple-import-sort';

export default [
    {
        files: ['**/*.ts', '**/*.tsx'],
        languageOptions: {
            parser: tsParser,
            parserOptions: {
                project: ['./tsconfig.json'],
                tsconfigRootDir: new URL('.', import.meta.url).pathname,
                sourceType: 'module',
            },
        },
        plugins: {
            '@typescript-eslint': tsPlugin,
            'simple-import-sort': simpleImportSort,
        },
        rules: {
            ...tsPlugin.configs.recommended.rules,
            'simple-import-sort/imports': ['error', {
                // Keep side-effect imports first, then everything else in a single group
                // (this disables blank lines between package and relative imports)
                groups: [
                    ['^\\u0000'],
                    ['^'],
                ],
            }],
            'simple-import-sort/exports': 'error',
        },
    },
]; 