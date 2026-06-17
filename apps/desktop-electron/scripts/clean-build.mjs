import { rm } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const outputDirs = ["dist", "out"].map((dir) => resolve(projectRoot, dir));

await Promise.all(outputDirs.map((dir) => rm(dir, { force: true, recursive: true })));
