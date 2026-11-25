import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

function readVersion(path: string): string {
  const json = JSON.parse(readFileSync(resolve(here, path), "utf8")) as { version?: string };
  if (!json.version) throw new Error(`${path} has no version field`);
  return json.version;
}

// The panel prints its version, so a build must not ship two different ones.
// tauri.conf.json is what the bundle reports as its version; package.json is
// what this constant is compiled from. They are bumped together by hand.
const pkgVersion = readVersion("package.json");
const bundleVersion = readVersion("src-tauri/tauri.conf.json");
if (pkgVersion !== bundleVersion) {
  throw new Error(
    `Version mismatch: package.json ${pkgVersion} vs src-tauri/tauri.conf.json ${bundleVersion}`,
  );
}

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  define: { __APP_VERSION__: JSON.stringify(pkgVersion) },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: { target: "safari15", minify: "esbuild", sourcemap: false },
});
