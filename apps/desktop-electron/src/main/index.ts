import { app, BrowserWindow } from "electron";
import { join } from "node:path";

const isDevelopment = process.env.VITE_DEV_SERVER_URL !== undefined;

async function createWindow(): Promise<void> {
  const window = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 960,
    minHeight: 640,
    backgroundColor: "#171717",
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: join(__dirname, "../preload/index.cjs")
    }
  });

  if (isDevelopment) {
    await window.loadURL(process.env.VITE_DEV_SERVER_URL as string);
    return;
  }

  await window.loadFile(join(__dirname, "../renderer/index.html"));
}

app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    void createWindow();
  }
});
