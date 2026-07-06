import { useMemo, useState } from "react";
import { AppShell } from "../components/AppShell";
import { HomePage } from "../features/home/HomePage";
import { ModelsPage } from "../features/models/ModelsPage";
import { ServerPage } from "../features/server/ServerPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { SpeakPage } from "../features/speak/SpeakPage";
import { TranscribePage } from "../features/transcribe/TranscribePage";
import { VoicesPage } from "../features/voices/VoicesPage";
import { mockRuntime } from "../lib/mockData";
import type { PageId } from "./navigation";

export function App() {
  const [activePage, setActivePage] = useState<PageId>("speak");
  const runtime = useMemo(() => mockRuntime, []);

  return (
    <AppShell activePage={activePage} onNavigate={setActivePage} runtime={runtime}>
      {activePage === "home" && <HomePage runtime={runtime} />}
      {activePage === "models" && <ModelsPage models={runtime.models} />}
      {activePage === "voices" && <VoicesPage voices={runtime.voices} />}
      {activePage === "speak" && <SpeakPage models={runtime.models} voices={runtime.voices} onViewModels={() => setActivePage("models")} />}
      {activePage === "transcribe" && <TranscribePage />}
      {activePage === "server" && <ServerPage runtime={runtime} />}
      {activePage === "settings" && <SettingsPage runtime={runtime} />}
    </AppShell>
  );
}
