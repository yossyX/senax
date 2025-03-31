import { CodegenConfig } from '@graphql-codegen/cli';
import * as fs from 'fs';

let env;
try {
  env = fs.readFileSync("../.env", 'utf-8');
} catch (err) {
  console.error('File read error:', err);
  process.exit(1);
}
const match = env.match(/^HOST_PORT=.*:(\d+)$/m);
const port = (match && match[1]) ? match[1] : '8080';

const config: CodegenConfig = {
  schema: `http://127.0.0.1:${port}/gql`,
  documents: ['src/**/*.tsx'],
  ignoreNoDocuments: true, // for better experience with the watcher
  generates: {
    "src/gql/": {
      preset: "client",
      presetConfig: {
        fragmentMasking: { unmaskFunctionName: 'getFragmentData' }
      },
      plugins: [],
      config: {
        enumsAsTypes: true
      }
    },
    'introspection.json': {
      plugins: ['introspection'],
      config: {
        minify: true,
        descriptions: false
      },
    },
  },
};

export default config;