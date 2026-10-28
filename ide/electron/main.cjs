const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');

let window;

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
