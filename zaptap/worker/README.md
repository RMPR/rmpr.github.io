# ZapTap API

A Cloudflare Worker that runs the daily puzzle and the leaderboard. It fits
comfortably in Cloudflare's free plan (100k requests/day; D1: 5M reads and
100k writes/day). A day of ZapTap for 50 people is roughly 600 requests.

What it does:

- Picks each day's five places with an HMAC of the date under a secret, so
  future puzzles can't be worked out from the public code.
- Hands out one place at a time and returns its answer only after you guess.
- Scores guesses itself (distance, round weights, country/continent bonus), so
  scores on the leaderboard can't be typed in by hand.
- Keeps one game per player per day. Days and the leaderboard roll over at
  midnight Oslo time.

Players have no account: the browser makes a random token and the Worker
stores its SHA-256 hash with the player's name. Names are unique.

## Deploy (about 5 minutes)

You need a free Cloudflare account and Node 18+.

```sh
cd zaptap/worker
npm install
npx wrangler login

# 1. Database: copy the printed database_id into wrangler.toml
npx wrangler d1 create zaptap
npx wrangler d1 execute zaptap --remote --file=schema.sql

# 2. Secret seed for the daily puzzles (any long random string)
openssl rand -hex 32 | npx wrangler secret put SEED_SECRET

# 3. Ship it
npx wrangler deploy
```

`wrangler deploy` prints the URL, e.g. `https://zaptap.<you>.workers.dev`.
Put it in `zaptap/config.js`:

```js
window.ZAPTAP_API = "https://zaptap.<you>.workers.dev";
```

Commit and push; the game switches to the server-run daily and shows the
leaderboard.

Keep `SEED_SECRET` secret and don't change it: changing it changes today's
puzzle under people's feet.

If the site is served from another domain, add it to `ALLOWED_ORIGINS` in
`wrangler.toml` and deploy again.

## Develop locally

```sh
echo 'SEED_SECRET="dev"' > .dev.vars
npx wrangler d1 execute zaptap --local --file=schema.sql
npx wrangler dev
```

Then set `window.ZAPTAP_API = "http://localhost:8787"` in `config.js` and serve
the site (`bundle exec jekyll serve`, which `ALLOWED_ORIGINS` already allows).

## Updating places

`src/places.json` is generated together with `../places.js` by
`../tools/build.js`. Redeploy the Worker after regenerating it.

## API

| Method | Path | Body | Returns |
| --- | --- | --- | --- |
| POST | `/api/state` | `{token}` | today's played rounds (with answers) and the next place |
| POST | `/api/guess` | `{token, day, round, lon, lat}` | the scored round, its answer, and the next place |
| POST | `/api/name` | `{token, name}` | the saved name, or `409 name_taken` |
| GET | `/api/leaderboard?day=YYYY-MM-DD` | | finished games for that day, best first |
