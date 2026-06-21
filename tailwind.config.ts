import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{vue,ts}"],
  theme: {
    extend: {
      colors: {
        surface: "#0F172A",
        panel: "#111827",
        line: "#263449",
        electric: "#2563EB",
        mint: "#10B981",
        amber: "#F59E0B",
      },
      fontFamily: {
        sans: ['"Microsoft YaHei UI"', "Inter", "system-ui", "sans-serif"],
        mono: ['"Cascadia Mono"', '"SFMono-Regular"', "ui-monospace", "monospace"],
      },
      boxShadow: {
        glass: "0 18px 50px rgba(15, 23, 42, 0.28)",
      },
    },
  },
  plugins: [],
} satisfies Config;
