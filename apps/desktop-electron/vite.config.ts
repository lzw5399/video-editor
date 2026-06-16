import { builtinModules } from "node:module";
import { resolve } from "node:path";

import react from "@vitejs/plugin-react";
import { defineConfig, type UserConfig } from "vite";

const rootDir = __dirname;
const external = ["electron", ...builtinModules, ...builtinModules.map((moduleName) => `node:${moduleName}`)];

function mainConfig(): UserConfig {
  return {
    root: rootDir,
    build: {
      emptyOutDir: false,
      lib: {
        entry: resolve(rootDir, "src/main/index.ts"),
        formats: ["cjs"],
        fileName: () => "index.cjs"
      },
      outDir: resolve(rootDir, "dist/main"),
      rollupOptions: {
        external
      }
    }
  };
}

function preloadConfig(): UserConfig {
  return {
    root: rootDir,
    build: {
      emptyOutDir: false,
      lib: {
        entry: resolve(rootDir, "src/preload/index.ts"),
        formats: ["cjs"],
        fileName: () => "index.cjs"
      },
      outDir: resolve(rootDir, "dist/preload"),
      rollupOptions: {
        external
      }
    }
  };
}

function rendererConfig(): UserConfig {
  return {
    root: rootDir,
    plugins: [react()],
    build: {
      emptyOutDir: false,
      outDir: resolve(rootDir, "dist/renderer")
    }
  };
}

export default defineConfig(({ mode }) => {
  if (mode === "main") {
    return mainConfig();
  }
  if (mode === "preload") {
    return preloadConfig();
  }
  return rendererConfig();
});
