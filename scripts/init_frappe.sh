#!/bin/bash

# Wait for DB
echo "Waiting for MariaDB..."
while ! mysqladmin ping -h"$DB_HOST" --silent; do
    sleep 1
done

# Initialize Bench if not exists (this is usually done by the image, but we need to ensure site creation)
cd /home/frappe/frappe-bench

# Check if site already exists
if [ ! -d "sites/$SITE_NAME" ]; then
    echo "Creating new site $SITE_NAME..."
    # Force reinstall if needed, set admin password
    bench new-site $SITE_NAME --force --mariadb-root-password $DB_ROOT_PASSWORD --admin-password $ADMIN_PASSWORD --no-mariadb-socket

    echo "Installing grafoso_saas app..."
    bench --site $SITE_NAME install-app grafoso_saas

    echo "Enabling scheduler..."
    bench --site $SITE_NAME enable-scheduler

    echo "Site created and app installed."
else
    echo "Site $SITE_NAME already exists."
    # Optional: Run migrations on startup
    echo "Running migrations..."
    bench --site $SITE_NAME migrate
fi

# Start the server (development mode for simplicity in this MVP, or production via supervisor)
# utilizing bench start which handles web, worker, etc.
# Ideally for production we use supervisor, but for "docker compose up" MVP, bench start is easiest to see logs.
bench start
