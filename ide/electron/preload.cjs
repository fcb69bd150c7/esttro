const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('api', {
  readFile: (filePath) => ipcRenderer.invoke('readFile', filePath),
  writeFile: (filePath, text) => ipcRenderer.invoke('writeFile', filePath, text),
});
