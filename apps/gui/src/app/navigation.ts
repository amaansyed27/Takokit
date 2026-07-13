import {
  AudioLines,
  Box,
  Boxes,
  History,
  House,
  Library,
  Server,
  Settings,
  Speech,
  User
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type PageId =
  | "home"
  | "models"
  | "runners"
  | "library"
  | "voices"
  | "speak"
  | "transcribe"
  | "history"
  | "diagnostics"
  | "settings";

export type NavItem = {
  id: PageId;
  label: string;
  icon: LucideIcon;
};

export const navItems: NavItem[] = [
  { id: "home", label: "Home", icon: House },
  { id: "models", label: "Models", icon: Box },
  { id: "runners", label: "Runners", icon: Boxes },
  { id: "library", label: "Library", icon: Library },
  { id: "voices", label: "Voices", icon: User },
  { id: "speak", label: "Speak", icon: AudioLines },
  { id: "transcribe", label: "Transcribe", icon: Speech },
  { id: "history", label: "History", icon: History },
  { id: "diagnostics", label: "Diagnostics", icon: Server },
  { id: "settings", label: "Settings", icon: Settings }
];
