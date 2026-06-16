import { contextBridge, ipcRenderer } from "electron";

import type { CommandEnvelope } from "../generated/CommandEnvelope";

contextBridge.exposeInMainWorld("videoEditorCore", {
  ping: () => ipcRenderer.invoke("core:ping"),
  version: () => ipcRenderer.invoke("core:version"),
  executeCommand: (command: CommandEnvelope) => ipcRenderer.invoke("core:executeCommand", command)
});
