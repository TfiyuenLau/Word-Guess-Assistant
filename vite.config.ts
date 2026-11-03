import { defineConfig, type Plugin } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

// Safe wrapper for Tailwind CSS Vite plugin to prevent non-CSS files (like Vue SFCs) from being parsed as CSS
function safeTailwind(): Plugin[] {
  const plugins = tailwindcss();
  return plugins.map((plugin) => {
    if (
      plugin.name === "@tailwindcss/vite:generate:serve" ||
      plugin.name === "@tailwindcss/vite:generate:build"
    ) {
      const origTransform = plugin.transform;
      if (
        typeof origTransform === "object" &&
        origTransform !== null &&
        "handler" in origTransform
      ) {
        const origHandler = (origTransform as any).handler;
        return {
          ...plugin,
          transform: {
            ...origTransform,
            async handler(this: any, code: string, id: string) {
              if (
                code.includes("<script") ||
                code.includes("<template") ||
                (id.endsWith(".vue") && !id.includes("type=style") && !id.includes("&lang.css"))
              ) {
                return null;
              }
              return origHandler.call(this, code, id);
            },
          },
        };
      }
    }
    return plugin;
  });
}

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [vue(), ...safeTailwind()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));

