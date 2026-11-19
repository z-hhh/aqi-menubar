import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import i18n, { resolveLanguage } from "./i18n";
import { bridge } from "./bridge";
import type { AppState } from "./types";
import Panel from "./Panel";
import Settings from "./Settings";

type View = "panel" | "settings";

export default function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [view, setView] = useState<View>("panel");
  const { t } = useTranslation();

  useEffect(() => {
    let alive = true;
    bridge
      .getState()
      .then((next) => {
        if (alive) setState(next);
      })
      .catch((err) => console.error("getState failed", err));
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    void bridge.onState((next) => setState(next)).then((un) => unlisteners.push(un));
    void bridge.onOpenSettings(() => setView("settings")).then((un) =>
      unlisteners.push(un),
    );
    return () => unlisteners.forEach((un) => un());
  }, []);

  const language = state?.config.language;
  useEffect(() => {
    if (language === undefined) return;
    const target = resolveLanguage(language);
    if (i18n.language !== target) void i18n.changeLanguage(target);
  }, [language]);

  if (!state) {
    return (
      <div className="shell">
        <div className="loading">{t("panel.loading")}</div>
      </div>
    );
  }

  return (
    <div className="shell">
      {view === "settings" ? (
        <Settings
          initial={state.config}
          onSaved={(next) => {
            setState(next);
            setView("panel");
          }}
          onBack={() => setView("panel")}
        />
      ) : (
        <Panel state={state} onOpenSettings={() => setView("settings")} />
      )}
    </div>
  );
}
