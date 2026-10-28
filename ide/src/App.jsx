import React, { useState, useEffect } from 'react';
import Editor from '@monaco-editor/react';
import { initEsttro } from './language/esttroMonaco';
import { executeEsttro } from './language/esttroRunner';

const DEFAULT_CODE = `const LIMIT = 5

print("Привет из Esttro!\\n")

function getSquare(n: int) -> int {
    return n * n
}

for i = 1, LIMIT {
    let sq: int = getSquare(i)
    print("Число: { i }, Квадрат: { sq }\\n")
}

print("Готово!\\n")
`;

export default function App() {
  const [code, setCode] = useState(DEFAULT_CODE);
  const [output, setOutput] = useState('');
  const [status, setStatus] = useState('F5 для запуска кода.');

  useEffect(() => {
    if (window.api) {
      window.api.readFile('examples/main.est').then((content) => {
        if (content) setCode(content);
      });
    }
  }, []);

  async function runCode() {
    setStatus('Выполняется...');

    let result;
    if (window.api && window.api.runEsttro) {
      result = await window.api.runEsttro(code);
    } else {
      const jsRes = executeEsttro(code);
      const diagnostics = [];
      if (jsRes.diagnostics && jsRes.diagnostics.length > 0) {
        jsRes.diagnostics.forEach((d) => {
          diagnostics.push(`Строка ${d.line}, колонка ${d.col}: ${d.message}`);
        });
      }
      result = {
        success: jsRes.success,
        output: jsRes.output,
        diagnostics,
        durationMs: jsRes.durationMs,
      };
    }

    let text = result.output || '';
    if (result.diagnostics && result.diagnostics.length > 0) {
      if (text && !text.endsWith('\n')) text += '\n';
      text += 'Ошибки:\n';
      result.diagnostics.forEach((d) => {
        text += `${d}\n`;
      });
    }

    if (text && !text.endsWith('\n')) text += '\n';
    text += `Завершено за ${result.durationMs} мс`;
    setOutput(text);
    setStatus(result.success ? 'Готово' : 'Ошибка');
  }

  useEffect(() => {
    function handleKey(e) {
      if (e.key === 'F5') {
        e.preventDefault();
        runCode();
      }
    }
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [code]);

  return (
    <div className="app">
      <div className="main">
        <div className="column">
          <div className="title">Код</div>
          <Editor
            height="100%"
            language="esttro"
            theme="vs-dark"
            value={code}
            onChange={(val) => setCode(val || '')}
            onMount={(editor, monaco) => initEsttro(monaco)}
            options={{
              fontFamily: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
              fontSize: 15,
              lineHeight: 24,
              minimap: { enabled: false },
              automaticLayout: true,
              scrollBeyondLastLine: false,
            }}
          />
        </div>

        <div className="column output-column">
          <div className="title">Вывод</div>
          <pre className="output">{output || 'F5 для запуска кода.'}</pre>
        </div>
      </div>

      <div className="footer">
        <span>{status}</span>
        <span>Esttro | UTF-8</span>
      </div>
    </div>
  );
}
