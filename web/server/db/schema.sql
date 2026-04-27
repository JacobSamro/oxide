-- Oxide admin schema (SQLite)
-- Tables: PascalCase singular; fields: camelCase

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS User (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  email         TEXT NOT NULL UNIQUE,
  name          TEXT NOT NULL,
  passwordHash  TEXT NOT NULL,
  isAdmin       INTEGER NOT NULL DEFAULT 0,
  createdAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  updatedAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);

CREATE TABLE IF NOT EXISTS Workspace (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  slug          TEXT NOT NULL UNIQUE,
  name          TEXT NOT NULL,
  description   TEXT,
  ownerId       INTEGER NOT NULL,
  createdAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  updatedAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  FOREIGN KEY (ownerId) REFERENCES User(id)
);
CREATE INDEX IF NOT EXISTS idx_workspace_owner ON Workspace(ownerId);

CREATE TABLE IF NOT EXISTS Team (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  workspaceId   INTEGER NOT NULL,
  slug          TEXT NOT NULL,
  name          TEXT NOT NULL,
  description   TEXT,
  createdAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  updatedAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  UNIQUE (workspaceId, slug),
  FOREIGN KEY (workspaceId) REFERENCES Workspace(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_team_ws ON Team(workspaceId);

CREATE TABLE IF NOT EXISTS Member (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  workspaceId   INTEGER NOT NULL,
  teamId        INTEGER,
  userId        INTEGER NOT NULL,
  role          TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner','admin','member','viewer')),
  createdAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  updatedAt     TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  UNIQUE (workspaceId, teamId, userId),
  FOREIGN KEY (workspaceId) REFERENCES Workspace(id) ON DELETE CASCADE,
  FOREIGN KEY (teamId)      REFERENCES Team(id)      ON DELETE CASCADE,
  FOREIGN KEY (userId)      REFERENCES User(id)      ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_member_user ON Member(userId);

CREATE TABLE IF NOT EXISTS Session (
  id         TEXT PRIMARY KEY,
  userId     INTEGER NOT NULL,
  expiresAt  TEXT NOT NULL,
  createdAt  TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  FOREIGN KEY (userId) REFERENCES User(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_session_user ON Session(userId);

-- Runtime settings (S3, domain, SSL). One JSON document per key.
CREATE TABLE IF NOT EXISTS Setting (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updatedAt  TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);
