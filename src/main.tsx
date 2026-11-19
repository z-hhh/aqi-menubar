import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import { isMock } from "./bridge";

// In a plain browser, preview the panel at its real window size.
if (isMock) document.body.classList.add("mock-preview");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
