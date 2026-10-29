# ZapTap server

A Cloudflare Worker that serves the game, runs the daily puzzle and keeps the
team leaderboard. It sits behind Cloudflare Access, so people sign in with
their Zivid Microsoft account. Everything here fits Cloudflare's free plans:

- Workers: 100k requests/day (a day of ZapTap for 50 people is ~600).
- D1: 5M reads and 100k writes/day.
- Zero Trust (Access): free for up to 50 users.

What it does:

- Serves the game's static files (`../`, minus what `../.assetsignore` lists)
  on the same origin as the API, so the Access login cookie just works.
- Picks each day's five places with an HMAC of the date under a secret, so
  future puzzles can't be worked out from the public code.
- Hands out one place at a time and returns its answer only after you guess.
- Scores guesses itself (distance, round weights, country/continent bonus), so
  scores on the leaderboard can't be typed in by hand.
- One game per person per day. Days and the leaderboard roll over at midnight
  Oslo time.

Players are identified by the email in the Access JWT, which the Worker
verifies itself (signature, audience, issuer, expiry and `ALLOWED_EMAIL_DOMAIN`).
The database only stores a SHA-256 of the email and the display name, which
starts as the Microsoft account name and can be changed in the game.

## Deploy

You need a Cloudflare account and Node 18+. Dashboard menu names drift; the
steps below are what to look for.

### 1. Zero Trust and Microsoft sign-in

1. In the Cloudflare dashboard, open **Zero Trust**, pick a team name (this
   gives you `<team>.cloudflareaccess.com`) and choose the **Free** plan. It
   may ask for a payment method even for the free plan.
2. In **Settings → Authentication → Login methods**, add **Microsoft Entra
   ID** (Azure AD). It needs an app registration in Zivid's Entra tenant, with
   redirect URI `https://<team>.cloudflareaccess.com/cdn-cgi/access/callback`;
   copy its client ID, a client secret and the tenant (directory) ID into
   Cloudflare. Cloudflare's guide for this is "Microsoft Entra ID" under
   Zero Trust identity providers. Registering the app (or consenting to its
   permissions) may need someone from IT.

   No Entra access yet? The built-in **One-time PIN** login (a code emailed to
   your @zivid.com address) works in the meantime; the game doesn't care which
   login method Access uses.

### 2. The Worker

```sh
cd zaptap/worker
npm install
npx wrangler login

# Database: copy the printed database_id into wrangler.toml
npx wrangler d1 create zaptap
npx wrangler d1 execute zaptap --remote --file=schema.sql

# Secret seed for the daily puzzles (any long random string)
openssl rand -hex 32 | npx wrangler secret put SEED_SECRET

npx wrangler deploy
```

This prints the URL, e.g. `https://zaptap.<you>.workers.dev`. Until step 3 is
done the API answers every request with an error, on purpose.

### 3. Put Access in front of it

1. In **Workers & Pages → zaptap → Settings → Domains & Routes**, enable
   **Cloudflare Access** on the `workers.dev` route. Cloudflare creates an
   Access application and shows its **AUD** tag and your team domain.
2. Put both in `wrangler.toml` (`ACCESS_AUD`, `ACCESS_TEAM_DOMAIN`) and run
   `npx wrangler deploy` again.
3. In **Zero Trust → Access → Applications**, open the new application and
   check its policy: allow **Emails ending in** `@zivid.com`, with Microsoft
   Entra ID as the login method. (The Worker also rejects other domains via
   `ALLOWED_EMAIL_DOMAIN`.)

Open the workers.dev URL: you should get the Microsoft sign-in, then the game.

### 4. Point rmpr.xyz/zaptap at it

Put the Worker URL in `../config.js`:

```js
window.ZAPTAP_HOME = "https://zaptap.<you>.workers.dev";
```

Commit and push; the GitHub Pages copy then redirects there.

Keep `SEED_SECRET` secret and don't change it: changing it changes today's
puzzle under people's feet. Deploying again after changing the game files
updates the game too, as the Worker serves them.

## Develop locally

`wrangler dev` has no Access in front of it. For local testing only, set
`DEV_AUTH="header"` and pass the player's email in an `X-Dev-Email` header.
Never set `DEV_AUTH` on the deployed Worker.

```sh
printf 'SEED_SECRET="dev"\nDEV_AUTH="header"\n' > .dev.vars
npx wrangler d1 execute zaptap --local --file=schema.sql
npx wrangler dev
curl -X POST -H 'X-Dev-Email: you@zivid.com' localhost:8787/api/state
```

## Updating places

`src/places.json` is generated together with `../places.js` by
`../tools/build.js`. Deploy again after regenerating it.

## API

All endpoints need a signed-in player.

| Method | Path | Body | Returns |
| --- | --- | --- | --- |
| POST | `/api/state` | `{}` | today's played rounds (with answers) and the next place |
| POST | `/api/guess` | `{day, round, lon, lat}` | the scored round, its answer, and the next place |
| POST | `/api/name` | `{name}` | the new display name, or `409 name_taken` |
| GET | `/api/leaderboard?day=YYYY-MM-DD` | | finished games for that day, best first |
