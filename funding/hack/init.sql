CREATE TABLE IF NOT EXISTS "transactions" (
  "id"        INTEGER PRIMARY KEY AUTOINCREMENT,
  "date"      INTEGER NOT NULL,
  "subject"   TEXT NOT NULL,
  "note"      TEXT NOT NULL,
  "currency"  TEXT NOT NULL,
  "amount"    REAL NOT NULL,
  "ending"    REAL NOT NULL,
  "available" REAL NOT NULL,
  "service"   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS "subplans" (
  "id"        INTEGER PRIMARY KEY AUTOINCREMENT,
  "plan_id"   TEXT NOT NULL,
  "price"     TEXT NOT NULL,
  "default"   INTEGER NOT NULL, -- 0 or 1
  UNIQUE ("plan_id")
);

CREATE TABLE IF NOT EXISTS "subscriptions" (
  "id"          INTEGER PRIMARY KEY AUTOINCREMENT,
  "sub_plan_id" TEXT NOT NULL,
  "start_date"  INTEGER NOT NULL,
  "end_date"    INTEGER NOT NULL
);
