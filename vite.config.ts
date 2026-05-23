import path from "path";
import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Dev proxy target resolution:
//   ANIMUS_DEV_PROXY_TARGET — full URL of the GraphQL transport plugin (default localhost:8081)
// At build time, configure the runtime API endpoint via:
//   VITE_ANIMUS_GRAPHQL_URL — explicit GraphQL HTTP URL
//   VITE_ANIMUS_GRAPHQL_WS_URL — explicit GraphQL websocket URL
//   VITE_ANIMUS_API_ORIGIN — origin only (/graphql appended at runtime)
// See src/lib/graphql/client.ts for the full precedence order.

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const proxyTarget = env.ANIMUS_DEV_PROXY_TARGET ?? "http://localhost:8081";

  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },
    server: {
      port: 5174,
      proxy: {
        "/graphql/ws": {
          target: proxyTarget,
          ws: true,
        },
        "/graphql": {
          target: proxyTarget,
        },
      },
    },
    test: {
      environment: "jsdom",
      setupFiles: [],
      globals: true,
      include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    },
    build: {
      outDir: "dist",
      emptyOutDir: true,
      cssCodeSplit: true,
      chunkSizeWarningLimit: 240,
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes("node_modules/react-router")) {
              return "routing-vendor";
            }
            if (id.includes("node_modules/react") || id.includes("node_modules/scheduler")) {
              return "react-vendor";
            }
            return undefined;
          },
        },
      },
    },
  };
});
