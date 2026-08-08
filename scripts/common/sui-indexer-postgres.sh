#!/bin/bash
set -e

# --- Configuration Variables ---
PG_USER="postgres"
PG_DB="sui_indexer"
PG_PASSWORD="postgrespw"
PG_VERSION=$(dpkg-query -W -f='${Version}\n' postgresql | cut -d'.' -f1)

if [[ -z "$PG_VERSION" ]]; then
    echo "PostgreSQL is not installed or version could not be determined. Proceeding with installation."
fi

# --- Helper functions ---
# Function to execute psql commands as the postgres user
run_psql() {
    sudo -u postgres psql -c "$1"
}

# --- 1. Install PostgreSQL (if not already installed) ---
if ! command -v psql &> /dev/null
then
    echo "Installing PostgreSQL..."
    sudo apt update
    sudo apt install -y postgresql postgresql-contrib
    echo "PostgreSQL installation complete."
else
    echo "PostgreSQL is already installed."
fi

# Get the major version of PostgreSQL
PG_VERSION=$(dpkg-query -W -f='${Version}\n' postgresql | cut -d'.' -f1)

# Ensure the service is running
echo "Checking PostgreSQL service status..."
if ! systemctl is-active --quiet postgresql; then
    echo "PostgreSQL service is not running. Starting it now..."
    sudo systemctl start postgresql
    sudo systemctl enable postgresql
    sleep 5 # Give the service time to start
else
    echo "PostgreSQL service is already running."
fi

# --- 2. Setup user and password ---
echo "Setting password for user '$PG_USER'..."
run_psql "ALTER USER $PG_USER WITH ENCRYPTED PASSWORD '$PG_PASSWORD';"

# --- 3. Create database if it doesn't exist ---
echo "Creating database '$PG_DB' if it doesn't exist..."
if ! run_psql "\\l" | grep -q "$PG_DB"; then
    run_psql "CREATE DATABASE $PG_DB;"
    echo "Database '$PG_DB' created successfully."
else
    echo "Database '$PG_DB' already exists. Skipping creation."
fi

# --- 4. Configure authentication for local connections ---
# The default `peer` authentication method on a fresh install prevents
# password-based connections. We will modify `pg_hba.conf`.
echo "Configuring pg_hba.conf for password authentication..."

# Path to the config file depends on the version
HBA_CONF="/etc/postgresql/$PG_VERSION/main/pg_hba.conf"
if [[ -f "$HBA_CONF" ]]; then
    # Replace `peer` with `md5` for local connections
    sudo sed -i "s/local\s\+all\s\+all\s\+peer/local   all             all                     md5/" "$HBA_CONF"
else
    echo "Warning: pg_hba.conf not found at $HBA_CONF. Manual configuration may be required."
fi

# --- 5. Restart PostgreSQL to apply changes ---
echo "Restarting PostgreSQL service to apply configuration changes..."
sudo systemctl restart postgresql
echo "PostgreSQL setup complete. You can now connect using the provided db_url."