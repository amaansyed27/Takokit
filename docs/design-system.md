# Design System

Takokit should feel like a serious local developer and creator tool: calm, paper-like, practical, and precise.

## Product Surface

- Use "local web GUI" or "browser GUI".
- Do not use desktop-app or Tauri wording.
- Do not add fake OS chrome.
- Show mock mode and not-implemented runner states honestly.

## Palette

| Token | Value |
| --- | --- |
| Background | `#F7F2E8` |
| Surface | `#FFFDF7` |
| Surface muted | `#EFE7DA` |
| Text primary | `#1F1D1A` |
| Text secondary | `#6F675E` |
| Border | `#D8CCBA` |
| Accent | `#6F8F72` |
| Accent dark | `#4F6F52` |
| Danger | `#A9564A` |

Use restrained borders. Avoid heavy shadows.

## Typography

- Use Orbitron as the identity/display font.
- Use Space Mono as the body and UI font.
- Keep headings direct and compact.
- Do not use oversized marketing hero type inside tool panels.
- Keep letter spacing at `0`.

## Component Rules

- Sidebar navigation uses icons plus labels.
- Tables and lists are preferred over card grids for model, runner, and voice registries.
- Forms should be direct and visibly functional.
- Status indicators should be compact.
- Avoid cards inside cards.
- Motion should be restrained: 120-240ms transitions, opacity/translate/clip reveals, and reduced-motion support.

## Avoid

- AI-purple gradients.
- Dark neon SaaS styling.
- Fake analytics dashboards.
- Fake OS title bars or traffic-light controls.
- Placeholder model inference claims.
- Overwrapped component trees where a simple feature component is enough.
