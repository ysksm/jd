#!/bin/bash
set -e

if [ -z "$DEPLOY_PATH" ]; then
    echo "Error: DEPLOY_PATH environment variable is not set"
    exit 1
fi

# Backup existing config files if they exist
CONFIG_BACKUP=""
SETTINGS_BACKUP=""

if [ -f "$DEPLOY_PATH/config.toml" ]; then
    CONFIG_BACKUP=$(mktemp)
    cp "$DEPLOY_PATH/config.toml" "$CONFIG_BACKUP"
    echo "Backed up config.toml to $CONFIG_BACKUP"
fi

if [ -f "$DEPLOY_PATH/data/settings.json" ]; then
    SETTINGS_BACKUP=$(mktemp)
    cp "$DEPLOY_PATH/data/settings.json" "$SETTINGS_BACKUP"
    echo "Backed up settings.json to $SETTINGS_BACKUP"
fi

pgrep -f 'jira-db-web .*--config' | xargs kill || true
./scripts/deploy.sh "$DEPLOY_PATH"

# Restore backed up config files
if [ -n "$CONFIG_BACKUP" ]; then
    cp "$CONFIG_BACKUP" "$DEPLOY_PATH/config.toml"
    rm "$CONFIG_BACKUP"
    echo "Restored config.toml"
fi

if [ -n "$SETTINGS_BACKUP" ]; then
    cp "$SETTINGS_BACKUP" "$DEPLOY_PATH/data/settings.json"
    rm "$SETTINGS_BACKUP"
    echo "Restored settings.json"
fi

"$DEPLOY_PATH/run.sh" &
