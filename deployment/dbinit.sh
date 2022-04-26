set -e

echo "start bootstrapping database"

echo "downloading bootstrap database file"
curl https://data.bgpkit.com/broker/bgpkit_broker_prod_postgres.gz -o /tmp/data.gz --silent

echo "bootstraping data"
gunzip < /tmp/data.gz | PGPASSWORD=${POSTGRES_PASSWORD} psql --username "${POSTGRES_USER}" --dbname "${POSTGRES_DB}" > /dev/null

echo "clean up temporary file"
rm /tmp/data.gz

echo "bootstrap complete!"
