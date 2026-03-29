import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { IndexBootstrap } from "../pages/IndexBootstrap";
import "../styles/foundation.css";

const rootElement = document.getElementById("react-root");

if (!rootElement) {
  throw new Error("Missing #react-root mount node.");
}

createRoot(rootElement).render(
  <StrictMode>
    <IndexBootstrap />
  </StrictMode>,
);
