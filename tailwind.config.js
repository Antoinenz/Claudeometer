/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          DEFAULT: "#111111",
          raised: "#1a1a1a",
          overlay: "#222222",
        },
        border: "#2a2a2a",
        accent: "#d97706",
      },
      fontFamily: {
        sans: ["-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
        mono: ["SF Mono", "Consolas", "monospace"],
      },
    },
  },
  plugins: [],
};
