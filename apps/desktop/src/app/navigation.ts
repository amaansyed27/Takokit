import {
  AudioLines,
  Box,
  House,
  Server,
  Settings,
  Speech,
  User
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type PageId = "home" | "models" | "voices" | "speak" | "transcribe" | "server" | "settings";

export type NavItem = {
  id: PageId;
  label: string;
  icon: LucideIcon;
};

export const navItems: NavItem[] = [
  { id: "home", label: "Home", icon: House },
  { id: "models", label: "Models", icon: Box },
  { id: "voices", label: "Voices", icon: User },
  { id: "speak", label: "Speak", icon: AudioLines },
  { id: "transcribe", label: "Transcribe", icon: Speech },
  { id: "server", label: "Server", icon: Server },
  { id: "settings", label: "Settings", icon: Settings }
];
