CREATE TABLE IF NOT EXISTS vouches (
    voucher TEXT NOT NULL,
    vouchee TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    PRIMARY KEY (voucher, vouchee)
);
CREATE INDEX IF NOT EXISTS voucher_idx ON vouches(voucher);
CREATE INDEX IF NOT EXISTS vouchee_idx ON vouches(vouchee);

CREATE TABLE IF NOT EXISTS proofs (
    user TEXT PRIMARY KEY,
    moderator TEXT NOT NULL,
    amount INTEGER NOT NULL,
    proof_id INTEGER NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS genesis (
    user TEXT PRIMARY KEY,
    balance INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS moderator_penalties (
    user TEXT PRIMARY KEY,
    moderator TEXT NOT NULL,
    amount INTEGER NOT NULL,
    proof_id INTEGER NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS forget_penalties (
    user TEXT NOT NULL,
    forgotten TEXT NOT NULL,
    amount INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    PRIMARY KEY (user, forgotten)
);
CREATE INDEX IF NOT EXISTS forget_penalties_idx ON forget_penalties(user);

CREATE TABLE IF NOT EXISTS admins (
    user TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS moderators (
    user TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS nonces (
    user TEXT PRIMARY KEY,
    used_nonce INTEGER NOT NULL
);
