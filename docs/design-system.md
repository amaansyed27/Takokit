# Design System

Takokit should feel like a serious local developer and creator tool: calm, paper-like, practical, and precise.

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

## Spacing

Use a simple 4px-based scale:

```txt
4, 8, 12, 16, 20, 24, 32, 40, 52
```

Dense tools can use 8-12px internal spacing. Page sections should breathe with 20-32px gaps.

## Typography

- Use system sans-serif fonts by default.
- Keep headings direct and compact.
- Do not use oversized marketing hero type inside tool panels.
- Use monospace only for paths, commands, API routes, and IDs.
- Keep letter spacing at `0`.

## Radius

- Controls and panels: `6px` to `8px`.
- Pills only for compact status labels.
- Avoid excessive rounding and nested framed containers.

## Component Rules

- Sidebar navigation uses icons plus labels.
- Tables and lists are preferred over card grids for model and voice registries.
- Forms should be direct and visibly functional.
- Status indicators should be compact.
- Avoid cards inside cards.
- Do not hardcode macOS, Windows, or Linux window chrome in React components. Platform chrome belongs in Tauri/window configuration or platform-aware shell code, not product UI.
- Motion should be restrained: 120-240ms transitions, opacity/translate/clip reveals, and reduced-motion support.

## Avoid

- AI-purple gradients.
- Dark neon SaaS styling.
- Fake analytics dashboards.
- Fake OS title bars or traffic-light controls.
- Random stock imagery.
- Large decorative cards for tiny facts.
- Placeholder model inference claims.
- Overwrapped component trees where a simple feature component is enough.
