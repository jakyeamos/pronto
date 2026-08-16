import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QualityRunnerCaseStudy } from "./components/QualityRunnerCaseStudy";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Quality Runner preview root is missing");
}

createRoot(root).render(
  <StrictMode>
    <main className="main-content quality-runner-preview">
      <div className="content-scroll">
        <QualityRunnerCaseStudy onBack={() => undefined} />
      </div>
    </main>
  </StrictMode>,
);
