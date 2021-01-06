CREATE TABLE IF NOT EXISTS "transactions" (
  "id"        SMALLSERIAL PRIMARY KEY,
  "date"      INTEGER NOT NULL,
  "subject"   TEXT NOT NULL,
  "note"      TEXT,
  "currency"  TEXT NOT NULL,
  "amount"    REAL NOT NULL,
  "ending"    REAL NOT NULL,
  "available" REAL NOT NULL,
  "service"   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS "subplans" (
  "id"        SMALLSERIAL PRIMARY KEY,
  "plan_id"   TEXT NOT NULL,
  "price"     TEXT NOT NULL,
  "default"   BOOLEAN NOT NULL,
  UNIQUE ("plan_id")
);

CREATE TABLE IF NOT EXISTS "subscriptions" (
  "id"        SMALLSERIAL PRIMARY KEY,
  "sub_plan_id" TEXT NOT NULL,
  "start_date"  INTEGER NOT NULL,
  "end_date"    INTEGER NOT NULL
);
