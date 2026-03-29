import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { NoAccessPage } from "../pages/NoAccessPage";
import "../styles/foundation.css";

const rootElement = document.getElementById("react-root");

if (rootElement == null) {
  throw new Error("Missing #react-root mount node.");
}

createRoot(rootElement).render(
  <StrictMode>
    <NoAccessPage />
  </StrictMode>,
);
