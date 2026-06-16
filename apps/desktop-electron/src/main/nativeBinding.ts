import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope } from "../generated/CommandResultEnvelope";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };

type NativeBinding = {
  ping: () => CommandResultEnvelope<PingResponse>;
  version: () => CommandResultEnvelope<VersionResponse>;
  executeCommand: (command: CommandEnvelope) => CommandResultEnvelope<unknown>;
};

const requireNative = createRequire(__filename);
const MAX_LOAD_ERROR_LENGTH = 600;

let cachedBinding: NativeBinding | null | undefined;
let cachedLoadError: string | null = null;

export function ping(): CommandResultEnvelope<PingResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("ping");
  }
  return binding.ping();
}

export function version(): CommandResultEnvelope<VersionResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("version");
  }
  return binding.version();
}

export function executeCommand(command: CommandEnvelope): CommandResultEnvelope<unknown> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError(command.command);
  }
  return binding.executeCommand(command);
}

function loadNativeBinding(): NativeBinding | null {
  if (cachedBinding !== undefined) {
    return cachedBinding;
  }

  const bindingPath = resolveNativeBindingPath();
  try {
    const loaded = requireNative(bindingPath) as Partial<NativeBinding>;
    if (
      typeof loaded.ping !== "function" ||
      typeof loaded.version !== "function" ||
      typeof loaded.executeCommand !== "function"
    ) {
      throw new Error("Native binding does not expose ping, version, and executeCommand");
    }

    cachedBinding = {
      ping: loaded.ping,
      version: loaded.version,
      executeCommand: loaded.executeCommand
    };
    cachedLoadError = null;
    return cachedBinding;
  } catch (error) {
    cachedBinding = null;
    cachedLoadError = boundErrorMessage(error);
    return null;
  }
}

function resolveNativeBindingPath(): string {
  if (process.env.VE_NATIVE_BINDING_PATH !== undefined) {
    return process.env.VE_NATIVE_BINDING_PATH;
  }

  const packageRootNativeEntry = join(__dirname, "../../native/index.cjs");
  if (existsSync(packageRootNativeEntry)) {
    return packageRootNativeEntry;
  }

  return join(__dirname, "../native/index.cjs");
}

function bindingLoadError(command: string): CommandResultEnvelope<never> {
  return {
    ok: false,
    data: null,
    error: {
      kind: "internal",
      command,
      message: `Native binding failed to load: ${cachedLoadError ?? "unknown load failure"}`
    },
    events: []
  };
}

function boundErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return message.length > MAX_LOAD_ERROR_LENGTH
    ? `${message.slice(0, MAX_LOAD_ERROR_LENGTH)}...`
    : message;
}
