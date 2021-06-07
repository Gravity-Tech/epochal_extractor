CREATE TABLE pollers_data (
  id SERIAL PRIMARY KEY,
  block_id BIGINT NOT NULL,
  poller_id INT NOT NULL
);

INSERT INTO pollers_data (block_id, poller_id)
VALUES (7906478, 1);