# load all env variables from .env file
set -a
source .env
set +a
# run the docker compose command
docker compose -f docker-compose.yml up --build --force-recreate --remove-orphans
