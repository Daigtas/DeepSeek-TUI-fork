"use client";

import { useEffect } from "react";

type Theme = "dark" | "light" | "system";

function resolveTheme(theme: Theme): "dark" | "light" {
  if (theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return theme;
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  useEffect(() => {
    const applyTheme = (theme: Theme) => {
      const resolved = resolveTheme(theme);
      document.documentElement.dataset.theme = resolved;
      // Also update Tailwind's dark mode class
      document.documentElement.classList.toggle("dark", resolved === "dark");
    };

    // Load from API
    fetch("/api/settings")
      .then((r) => r.json())
      .then((d) => {
        if (d.preferences?.theme) {
          applyTheme(d.preferences.theme);
        }
      })
      .catch((err) => {
        console.warn("[Theme] failed to load theme preference:", err);
        applyTheme("system");
      });

    // Listen for system theme changes (when theme is "system")
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      // Re-fetch and re-apply
      fetch("/api/settings")
        .then((r) => r.json())
        .then((d) => {
          if (d.preferences?.theme) {
            applyTheme(d.preferences.theme);
          }
        })
        .catch((err) => console.warn("[Theme] failed to re-fetch theme:", err));
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  return <>{children}</>;
}
