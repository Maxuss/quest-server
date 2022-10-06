CREATE TABLE IF NOT EXISTS users(
    card_hash char(64) PRIMARY KEY UNIQUE NOT NULL,
    id UUID UNIQUE NOT NULL,
    username varchar(32) NOT NULL,
    telegram_chat_id int NOT NULL
);

CREATE TABLE IF NOT EXISTS users_reg_state(
    card_hash char(64) PRIMARY KEY UNIQUE NOT NULL,
    id UUID UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS lingering_quests(
    id UUID PRIMARY KEY UNIQUE NOT NULL,
    assigned_to char(64) NOT NULL,
    quest_name varchar(32) NOT NULL
)