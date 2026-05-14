import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // References CSS custom properties so theme toggle actually works.
        // The property values are defined in globals.css under :root/[data-theme="dark"]
        // and [data-theme="light"].
        bg: {
          DEFAULT: "var(--bg)",
          alt: "var(--bg-alt)",
          card: "var(--bg-card)",
          hover: "var(--bg-hover)",
        },
        border: "var(--border)",
        fg: {
          DEFAULT: "var(--fg)",
          dim: "var(--fg-dim)",
          faint: "var(--fg-faint)",
        },
        amber: {
          DEFAULT: "var(--amber)",
          light: "var(--amber-light)",
          dim: "var(--amber-dim)",
        },
        green: {
          DEFAULT: "var(--green)",
          light: "var(--green-light)",
          dim: "var(--green-dim)",
        },
        cyan: {
          DEFAULT: "var(--cyan)",
          light: "var(--cyan-light)",
          dim: "var(--cyan-dim)",
        },
        rose: {
          DEFAULT: "var(--rose)",
          light: "var(--rose-light)",
          dim: "var(--rose-dim)",
        },
      },
      fontFamily: {
        mono: [
          "JetBrains Mono", "Fira Code", "Cascadia Code", "Consolas",
          "Menlo", "monospace",
        ],
        sans: [
          "Inter", "system-ui", "-apple-system", "sans-serif",
        ],
      },
      borderRadius: {
        sm: "2px",
        DEFAULT: "4px",
        md: "6px",
      },
      boxShadow: {
        border: "0 0 0 1px var(--border)",
      },
      animation: {
        "pulse-dot": "pulse-dot 1.4s infinite ease-in-out",
        "slide-up": "slide-up 0.15s ease-out",
        "fade-in": "fade-in 0.1s ease-out",
        "cursor-blink": "cursor-blink 1s step-end infinite",
        "shimmer": "shimmer 1.4s ease-in-out infinite",
        "slide-up-fade": "slide-up-fade 0.2s ease-out both",
        "toast-in": "toast-in 0.2s ease-out both",
        "toast-out": "toast-out 0.15s ease-in both",
        "bounce-in": "bounce-in 0.25s ease-out both",
        "scale-press": "scale-press 0.15s ease-out",
      },
      keyframes: {
        "pulse-dot": {
          "0%, 80%, 100%": { opacity: "0" },
          "40%": { opacity: "1" },
        },
        "slide-up": {
          from: { opacity: "0", transform: "translateY(4px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
        "fade-in": {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        "cursor-blink": {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0" },
        },
        "shimmer": {
          "0%": { backgroundPosition: "-200% 0" },
          "100%": { backgroundPosition: "200% 0" },
        },
        "slide-up-fade": {
          from: { opacity: "0", transform: "translateY(8px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
        "toast-in": {
          from: { opacity: "0", transform: "translateY(-6px) scale(0.96)" },
          to: { opacity: "1", transform: "translateY(0) scale(1)" },
        },
        "toast-out": {
          from: { opacity: "1", transform: "translateY(0) scale(1)" },
          to: { opacity: "0", transform: "translateY(-6px) scale(0.96)" },
        },
        "bounce-in": {
          "0%": { opacity: "0", transform: "scale(0.8)" },
          "50%": { transform: "scale(1.04)" },
          "100%": { opacity: "1", transform: "scale(1)" },
        },
        "scale-press": {
          "0%": { transform: "scale(1)" },
          "50%": { transform: "scale(0.92)" },
          "100%": { transform: "scale(1)" },
        },
      },
    },
  },
  plugins: [],
};

export default config;
