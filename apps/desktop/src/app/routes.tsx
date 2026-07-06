import { HomePage } from "../features/home/HomePage";
import { ModelsPage } from "../features/models/ModelsPage";
import { ServerPage } from "../features/server/ServerPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { SpeakPage } from "../features/speak/SpeakPage";
import { TranscribePage } from "../features/transcribe/TranscribePage";
import { VoicesPage } from "../features/voices/VoicesPage";
import type { RuntimeSnapshot } from "../lib/types";
import type { PageId } from "./navigation";

export type RouteComponentProps = {
  runtime: RuntimeSnapshot;
  onNavigate: (page: PageId) => void;
};

export type AppRoute = {
  id: PageId;
  component: (props: RouteComponentProps) => JSX.Element;
};

export const routes: AppRoute[] = [
  { id: "home", component: HomePage },
  { id: "models", component: ModelsPage },
  { id: "voices", component: VoicesPage },
  { id: "speak", component: SpeakPage },
  { id: "transcribe", component: TranscribePage },
  { id: "server", component: ServerPage },
  { id: "settings", component: SettingsPage }
];

