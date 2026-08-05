# Companion site deployment

The canonical maintained public site is the React/Vite application in `site/`.
There is no second static website implementation.

## Vercel project settings

Use these settings for the existing Takokit website project:

```text
Root Directory: site
Framework Preset: Vite
Install Command: npm ci
Build Command: npm run build
Output Directory: dist
```

The committed `site/vercel.json` owns the SPA rewrite, registry API alias, installer
headers, and hashed-asset caching. The Vite build reads the canonical logo and registry
from the repository root. It can fall back to the deployed Git revision when Vercel
isolates the configured root directory.

## Previous production mismatch

The old production alias pointed to a manually uploaded static deployment built with
`takokit-library@0.1.0` and `node scripts/build.mjs`. It contained legacy HTML, CSS, and
JavaScript, had no current Git branch metadata, and Vercel reported no detected framework.
The maintained repository had already moved to React/Vite (`takokit-library@0.2.0`), but
production was not building that source. Moving the alias to a verified Vite deployment
removes that mismatch.

## Preview deployment from PowerShell

Run Vercel from the repository root because this is a monorepo project whose configured
Root Directory is `site`.

```powershell
cd D:\TheDawnlightGroup\DawnlightLabs\Takokit
git fetch origin
git switch slice/website-library-deployment-finish
git pull --ff-only origin slice/website-library-deployment-finish

npm ci --prefix site
npm run check --prefix site

npm install --global vercel@latest
vercel login
vercel link --repo
vercel pull --yes --environment=preview
vercel build
$PreviewUrl = (vercel deploy --prebuilt | Select-Object -Last 1).Trim()
$PreviewUrl
npm run verify:deployment --prefix site -- $PreviewUrl
```

During `vercel link --repo`, select the existing Takokit website project. Do not create a
second production project. If the linked project reports a different Root Directory,
change it to `site` before building.

If preview deployment protection is enabled, set the automation bypass secret before the
verification command:

```powershell
$env:VERCEL_AUTOMATION_BYPASS_SECRET = "YOUR_BYPASS_SECRET"
npm run verify:deployment --prefix site -- $PreviewUrl
```

## Production deployment

Only run this after the preview verification and manual checklist pass:

```powershell
cd D:\TheDawnlightGroup\DawnlightLabs\Takokit
vercel pull --yes --environment=production
vercel build --prod
$ProductionUrl = (vercel deploy --prebuilt --prod | Select-Object -Last 1).Trim()
$ProductionUrl
npm run verify:deployment --prefix site -- $ProductionUrl
```

A production deployment assigns the project's production domains. Confirm the URL and the
registry response before removing or redirecting any older alias.

## What the automated deployment check covers

`npm run verify:deployment --prefix site -- URL` verifies:

- homepage and direct SPA routes
- `/models` and legacy `/library` routes
- direct model and documentation routes
- Vite hashed assets and immutable cache headers
- `/v1/registry.json` and `/api/v1/registry`
- canonical logo and favicons
- PowerShell and shell installer files
- absence of legacy static build references

Browser interaction, accessibility, filter behaviour, and installer execution still require
manual testing.
