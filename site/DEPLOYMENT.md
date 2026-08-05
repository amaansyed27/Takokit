# Companion site deployment

The canonical maintained public site is the React/Vite application in this `site/` directory.

Vercel project settings must use:

```text
Root Directory: site
Framework Preset: Vite
Build Command: npm run build
Output Directory: dist
```

`site/vercel.json` owns the SPA rewrite, registry API rewrite, and immutable hashed-asset caching.

## Previous production mismatch

The production alias was attached to a manually uploaded static deployment built with
`takokit-library@0.1.0` and `node scripts/build.mjs`. That deployment contained legacy
HTML, CSS, and JavaScript assets and had no current Git branch metadata. The maintained
repository had already moved to React/Vite (`takokit-library@0.2.0`), but production was
not building that source. The Vercel project also reported no detected framework.

A branch preview must prove the Vite application, direct route refreshes, and registry
function before the production alias is moved. Do not retain or revive a second static
site implementation.
