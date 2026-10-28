export function initEsttro(monaco) {
  if (monaco.languages.getLanguages().some((l) => l.id === 'esttro')) {
    return;
  }

  monaco.languages.register({ id: 'esttro', extensions: ['.est'] });

  monaco.languages.setMonarchTokensProvider('esttro', {
    keywords: [
      'function', 'let', 'const', 'if', 'else', 'while', 'for',
      'return', 'break', 'continue', 'true', 'false',
    ],
    types: ['int', 'float', 'double', 'boolean', 'string', 'void'],
    builtins: ['print'],

    tokenizer: {
      root: [
        [/[a-zA-Z_]\w*/, {
          cases: {
            '@keywords': 'keyword',
            '@types': 'type',
            '@builtins': 'predefined',
            '@default': 'identifier',
          },
        }],
        [/\/\/.*$/, 'comment'],
        [/\d+/, 'number'],
        [/"([^"\\]|\\.)*"/, 'string'],
      ],
    },
  });
}
