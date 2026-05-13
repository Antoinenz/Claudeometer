import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// ── Strip webview tells ──────────────────────────────────────────────────────

// No right-click context menu
document.addEventListener("contextmenu", (e) => e.preventDefault());

// Mutable flags — updated at runtime by App when settings load/change.
const debugFlags = { devtools: false, webviewReload: false };
export function applyDebugFlags(flags: { devtools: boolean; webviewReload: boolean }) {
  debugFlags.devtools = flags.devtools;
  debugFlags.webviewReload = flags.webviewReload;
}

// Block browser-default keyboard shortcuts that don't belong in a native app.
document.addEventListener("keydown", (e) => {
  const ctrl = e.ctrlKey || e.metaKey;

  const blocked =
    (!debugFlags.devtools    && e.key === "F12") ||
    (!debugFlags.devtools    && ctrl && e.shiftKey && (e.key === "I" || e.key === "J")) ||
    e.key === "F5"                                ||  // hard reload — always blocked
    (ctrl && e.key === "r" && !e.shiftKey)        ||  // Ctrl+R → handled by App as refresh
    (!debugFlags.webviewReload && ctrl && e.shiftKey && e.key === "R") ||
    (ctrl && e.key === "p")  ||
    (ctrl && e.key === "s")  ||
    (ctrl && e.key === "u")  ||
    (ctrl && e.key === "f")  ||
    (ctrl && e.key === "g")  ||
    (ctrl && e.key === "j")  ||
    (e.altKey && (e.key === "ArrowLeft" || e.key === "ArrowRight"));

  if (blocked) e.preventDefault();
}, { capture: true });

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
