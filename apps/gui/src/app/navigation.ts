import {
  AudioLines,
  AudioWaveform,
  Box,
  Boxes,
  CircleGauge,
  History,
  House,
  Settings,
  Speech,
  UserRound
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type PageId =
  | "home"
  | "models"
  | "runners"
  | "voices"
  | "speak"
  | "transcribe"
  | "convert"
  | "history"
  | "diagnostics"
  | "settings";

export type NavItem = {
  id: PageId;
  label: string;
  icon: LucideIcon;
};

export type NavSection = {
  label?: string;
  items: NavItem[];
};

export const navSections: NavSection[] = [
  {
    items: [{ id: "home", label: "Home", icon: House }]
  },
  {
    label: "Create",
    items: [
      { id: "speak", label: "Speak", icon: AudioLines },
      { id: "transcribe", label: "Transcribe", icon: Speech },
      { id: "convert", label: "Convert voice", icon: AudioWaveform }
    ]
  },
  {
    label: "Library",
    items: [
      { id: "voices", label: "Voices", icon: UserRound },
      { id: "models", label: "Models", icon: Box },
      { id: "runners", label: "Runners", icon: Boxes }
    ]
  },
  {
    label: "System",
    items: [
      { id: "history", label: "History", icon: History },
      { id: "diagnostics", label: "Diagnostics", icon: CircleGauge },
      { id: "settings", label: "Settings", icon: Settings }
    ]
  }
];

export const navItems = navSections.flatMap((section) => section.items);
