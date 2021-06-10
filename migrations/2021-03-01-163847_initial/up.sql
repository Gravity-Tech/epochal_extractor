-- Your SQL goes here
CREATE TABLE extracted_data(
    id UUID PRIMARY KEY,
    base64Bytes VARCHAR NOT NULL,
    block_id BIGINT NOT NULL, 
    priority INT NOT NULL
);
