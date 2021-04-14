ethblock= 12239857
ftmblock=3711580
bscblock=6565725
docker pull badconfig/lp_withdraw_extractor
docker pull badconfig/gton_extractor
docker pull badconfig/graviton_subs_extractor 
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "CREATE TABLE extracted_data(id SERIAL PRIMARY KEY,base64Bytes VARCHAR NOT NULL);"
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "CREATE TABLE poller_states(id SERIAL PRIMARY KEY,num INT NOT NULL DEFAULT 0);"
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "DELETE FROM poller_states;"
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "DELETE FROM extracted_data;"
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "INSERT INTO poller_states(id,num) VALUES (1,${ethblock}),(2,${ethblock}),(3,${ethblock}),(4,${ethblock}),(5,${ethblock}),(6,${ethblock}),(7,${ethblock}),(8,${ethblock}),(9,${ethblock}),(10,${ethblock}),(11,${ethblock}),(12,${ethblock});"
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "INSERT INTO poller_states(id,num) VALUES (14,${ftmblock}),(15,${ftmblock}),(16,${ftmblock});"
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "INSERT INTO poller_states(id,num) VALUES (17,${bscblock}),(18,${bscblock}),(19,${bscblock}),(20,${bscblock});"
docker-compose up -d

docker exec -it postgres_db_transactions psql -U docker diesel_db -c "SELECT * FROM poller_states"
docker exec -it postgres_db_transactions psql -U docker diesel_db -c "SELECT * FROM extracted_data"
