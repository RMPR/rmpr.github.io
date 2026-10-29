-- ZapTap leaderboard. Apply with:
--   npx wrangler d1 execute zaptap --remote --file=schema.sql

CREATE TABLE IF NOT EXISTS players (
  id TEXT PRIMARY KEY,            -- sha256 of the browser's secret token
  name TEXT NOT NULL,
  name_key TEXT NOT NULL UNIQUE,  -- lower-cased name, so names are unique
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS guesses (
  day TEXT NOT NULL,              -- YYYY-MM-DD, Oslo time
  player TEXT NOT NULL REFERENCES players(id),
  round INTEGER NOT NULL,         -- 0..4
  lon REAL NOT NULL,
  lat REAL NOT NULL,
  km REAL NOT NULL,
  score INTEGER NOT NULL,         -- 0..100
  points INTEGER NOT NULL,        -- score x round weight
  bonus TEXT,                     -- 'country', 'continent' or NULL
  in_country INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (day, player, round)
);

CREATE INDEX IF NOT EXISTS guesses_by_day ON guesses (day);
