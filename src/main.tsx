import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// ── Strip webview tells ──────────────────────────────────────────────────────

// No right-click context menu
document.addEventListener("contextmenu", (e) => e.preventDefault());

// Block browser-default keyboard shortcuts that don't belong in a native app.
// Ctrl+R refresh is handled in App.tsx and should not trigger a page reload.
document.addEventListener("keydown", (e) => {
  const ctrl = e.ctrlKey || e.metaKey;
  const blocked =
    e.key === "F12" ||                          // DevTools
    e.key === "F5"  ||                          // Reload
    (ctrl && e.key === "r")  ||                 // Reload (Ctrl+R handled in App)
    (ctrl && e.shiftKey && e.key === "R") ||    // Hard reload
    (ctrl && e.key === "p")  ||                 // Print
    (ctrl && e.key === "s")  ||                 // Save page
    (ctrl && e.key === "u")  ||                 // View source
    (ctrl && e.key === "f")  ||                 // Find in page
    (ctrl && e.key === "g")  ||                 // Find next
    (ctrl && e.key === "j")  ||                 // Downloads
    (ctrl && e.shiftKey && e.key === "I") ||    // DevTools (alt)
    (ctrl && e.shiftKey && e.key === "J") ||    // Console
    (e.altKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")); // Browser back/forward

  if (blocked) e.preventDefault();
}, { capture: true });

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
