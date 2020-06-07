-- Your SQL goes here
CREATE TABLE user_tbl (
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL,
    password VARCHAR NOT NULL,
    salt VARCHAR NOT NULL,
    nickname VARCHAR NOT NULL,
    email VARCHAR,
    avatar VARCHAR,
)