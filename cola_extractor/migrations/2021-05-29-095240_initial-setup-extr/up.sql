CREATE TABLE poller_states (
    id SERIAL PRIMARY KEY,
    num BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE extracted_data(
    id UUID PRIMARY KEY,
    base64Bytes VARCHAR NOT NULL,
    block_id BIGINT NOT NULL, 
    priority INT NOT NULL
);

