CREATE TABLE state (
    id INTEGER PRIMARY KEY
    CHECK (id = 1),

    rev INTEGER NOT NULL
    CHECK (rev >= 0)
) STRICT;

INSERT INTO state (id, rev)
VALUES (1, 0);

CREATE TABLE node (
    id INTEGER PRIMARY KEY,

    -- What task is it?
    kind TEXT NOT NULL,

    -- What arguments are provided to it?
    key TEXT NOT NULL,

    -- Hash of its output
    output_hash BLOB NOT NULL
    CHECK (length(output_hash) = 32),

    -- Each task of a given version should be unique in this table
    UNIQUE (kind, key)
) STRICT;

CREATE TABLE dependency (
    -- What depends on this?
    parent INTEGER NOT NULL
    REFERENCES node(id)
    ON DELETE CASCADE,

    -- What does this depend on?
    dep INTEGER NOT NULL
    REFERENCES node(id)
	ON DELETE RESTRICT,

    -- What was the last observed hash of this dependency?
    -- If expected != dep.hash, then dep has changed.
    expected BLOB NOT NULL
    CHECK (length(expected) = 32),

    PRIMARY KEY (parent, dep),
    CHECK (parent != dep)
) STRICT;
