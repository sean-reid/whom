import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";
import { defineConfig } from "vitest/config";

const STYLE_HASH = "__STYLE_HASH__";

// The built stylesheet goes into index.html and its sha256 into the CSP in _headers.
function inlineStyle(): Plugin {
  let hash = "";
  let outDir = "dist";
  return {
    name: "whom:inline-style",
    apply: "build",
    configResolved(config) {
      outDir = config.build.outDir;
    },
    transformIndexHtml: {
      order: "post",
      handler(html, ctx) {
        const bundle = ctx.bundle ?? {};
        const sheets = Object.values(bundle).filter(
          (item) => item.type === "asset" && item.fileName.endsWith(".css"),
        );
        const sheet = sheets[0];
        if (sheets.length !== 1 || sheet?.type !== "asset") {
          throw new Error(`expected one stylesheet, found ${sheets.length}`);
        }
        const css =
          typeof sheet.source === "string" ? sheet.source : Buffer.from(sheet.source).toString();
        delete bundle[sheet.fileName];
        hash = createHash("sha256").update(css).digest("base64");
        const link = /\s*<link rel="stylesheet"[^>]*href="[^"]*\.css"[^>]*>/;
        if (!link.test(html)) throw new Error("no stylesheet link to inline");
        return html.replace(link, `\n    <style>${css}</style>`);
      },
    },
    closeBundle() {
      const path = join(outDir, "_headers");
      const headers = readFileSync(path, "utf8");
      if (!headers.includes(STYLE_HASH)) throw new Error(`${path} has no ${STYLE_HASH}`);
      writeFileSync(path, headers.replace(STYLE_HASH, hash));
    },
  };
}

export default defineConfig({
  plugins: [inlineStyle()],
  build: {
    target: "es2022",
    sourcemap: false,
    modulePreload: { polyfill: false },
  },
  server: {
    proxy: { "/api": "http://localhost:8787" },
  },
  test: {
    environment: "node",
    include: ["tests/unit/**/*.test.ts"],
  },
});
