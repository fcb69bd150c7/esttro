const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { execFile } = require('child_process');

let window;
let configuredBinaryPath = null;

for (let i = 0; i < process.argv.length; i++) {
  const arg = process.argv[i];
  if (arg.startsWith('--binary=')) {
    configuredBinaryPath = arg.substring('--binary='.length);
  } else if (arg === '--binary' && i + 1 < process.argv.length) {
    configuredBinaryPath = process.argv[i + 1];
  }
}

function resolveBinary() {
  if (configuredBinaryPath) {
    const resolved = path.resolve(process.cwd(), configuredBinaryPath);
    if (fs.existsSync(resolved)) return resolved;
    return configuredBinaryPath;
  }
  if (process.env.ESTTRO_BINARY) {
    const resolved = path.resolve(process.cwd(), process.env.ESTTRO_BINARY);
    if (fs.existsSync(resolved)) return resolved;
  }
  const defaults = [
    path.resolve(__dirname, '../../bin/esttro.exe'),
    path.resolve(__dirname, '../../target/release/esttro.exe'),
    path.resolve(__dirname, '../../target/debug/esttro.exe'),
  ];
  for (const c of defaults) {
    if (fs.existsSync(c)) return c;
  }
  return 'esttro.exe';
}

function createWindow() {
  window = new BrowserWindow({
    width: 1050,
    height: 700,
    title: 'Esttro',
    backgroundColor: '#1e1e1e',
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
    },
  });

  const distPath = path.join(__dirname, '../dist/index.html');
  if (fs.existsSync(distPath)) {
    window.loadFile(distPath);
  } else {
    window.loadURL('http://localhost:5173');
  }
}

app.whenReady().then(createWindow);

ipcMain.handle('readFile', (event, filePath) => {
  const fullPath = path.resolve(__dirname, '../../', filePath);
  if (fs.existsSync(fullPath)) {
    return fs.readFileSync(fullPath, 'utf8');
  }
  return '';
});

ipcMain.handle('writeFile', (event, filePath, text) => {
  const fullPath = path.resolve(__dirname, '../../', filePath);
  fs.writeFileSync(fullPath, text, 'utf8');
  return true;
});

ipcMain.handle('runEsttro', async (event, code) => {
  const binary = resolveBinary();
  const startTime = Date.now();

  const tempDir = app.getPath('temp');
  const tempFile = path.join(tempDir, `esttro_temp_${Date.now()}_${Math.random().toString(36).slice(2)}.est`);

  try {
    fs.writeFileSync(tempFile, code, 'utf8');

    return await new Promise((resolve) => {
      execFile(binary, [tempFile], { timeout: 10000, windowsHide: true }, (error, stdout, stderr) => {
        const durationMs = Date.now() - startTime;
        try {
          if (fs.existsSync(tempFile)) fs.unlinkSync(tempFile);
        } catch (_) {}

        if (error && error.code === 'ENOENT') {
          resolve({
            success: false,
            output: '',
            diagnostics: [
              `Бинарный файл компилятора не найден: ${binary}\nУкажите путь через параметр --binary="<путь к esttro.exe>"`
            ],
            durationMs,
          });
          return;
        }

        const outStr = (stdout || '').toString();
        const errStr = (stderr || '').toString().trim();

        const diagnostics = [];
        if (errStr) {
          const cleanedErr = errStr.replace(new RegExp(tempFile.replace(/\\/g, '\\\\'), 'g'), 'code.est');
          diagnostics.push(cleanedErr);
        }

        resolve({
          success: !error && !errStr,
          output: outStr,
          diagnostics,
          durationMs,
        });
      });
    });
  } catch (err) {
    return {
      success: false,
      output: '',
      diagnostics: [err.message],
      durationMs: Date.now() - startTime,
    };
  }
});
