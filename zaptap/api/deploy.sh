#!/usr/bin/env bash
# Creates or updates ZapTap on Azure: a Static Web App (Free plan) serving the
# game and its API, plus a storage account for the leaderboard tables.
# Safe to run again; it never changes the puzzle seed once set.
#
# Run from the repository root with the az CLI and Node installed. Signs in
# with the service principal in AZURE_CLIENT_ID / AZURE_CLIENT_SECRET /
# AZURE_TENANT_ID / AZURE_SUBSCRIPTION_ID if they're set, otherwise uses the
# current `az login` (e.g. in Azure Cloud Shell).
set -euo pipefail

RG=${ZAPTAP_RG:-zaptap-rg}
LOCATION=${ZAPTAP_LOCATION:-westeurope}
APP=${ZAPTAP_APP:-zaptap}
DOMAIN=${ZAPTAP_EMAIL_DOMAIN:-zivid.com}

if [ -n "${AZURE_CLIENT_ID:-}" ]; then
  az login --service-principal -u "$AZURE_CLIENT_ID" -p "$AZURE_CLIENT_SECRET" --tenant "$AZURE_TENANT_ID" --output none
  az account set --subscription "$AZURE_SUBSCRIPTION_ID"
fi

az group show --name "$RG" --output none 2>/dev/null || az group create --name "$RG" --location "$LOCATION" --output none

# Storage account names are global: derive one from the subscription id.
ST=${ZAPTAP_STORAGE_ACCOUNT:-zaptap$(az account show --query id -o tsv | tr -d '-' | cut -c1-12)}
if ! az storage account show --name "$ST" --resource-group "$RG" --output none 2>/dev/null; then
  echo "Creating storage account $ST"
  az storage account create --name "$ST" --resource-group "$RG" --location "$LOCATION" \
    --sku Standard_LRS --kind StorageV2 --min-tls-version TLS1_2 --allow-blob-public-access false --output none
fi
CONN=$(az storage account show-connection-string --name "$ST" --resource-group "$RG" -o tsv)

if ! az staticwebapp show --name "$APP" --resource-group "$RG" --output none 2>/dev/null; then
  echo "Creating static web app $APP"
  az staticwebapp create --name "$APP" --resource-group "$RG" --location "$LOCATION" --sku Free --output none
fi

# The seed picks the daily puzzles: create it once, keep it forever.
SEED=$(az staticwebapp appsettings list --name "$APP" --resource-group "$RG" --query "properties.SEED_SECRET" -o tsv 2>/dev/null || true)
[ -n "$SEED" ] || SEED=$(openssl rand -hex 32)
az staticwebapp appsettings set --name "$APP" --resource-group "$RG" --output none \
  --setting-names "SEED_SECRET=$SEED" "ZAPTAP_STORAGE=$CONN" "ALLOWED_EMAIL_DOMAIN=$DOMAIN"

# Deploy the game (zaptap/) and its API (zaptap/api/).
(cd zaptap/api && npm install --omit=dev --no-audit --no-fund)
TOKEN=$(az staticwebapp secrets list --name "$APP" --resource-group "$RG" --query "properties.apiKey" -o tsv)
npx --yes @azure/static-web-apps-cli@2 deploy ./zaptap \
  --api-location ./zaptap/api --api-language node --api-version 20 \
  --deployment-token "$TOKEN" --env production

echo "ZapTap is live at https://$(az staticwebapp show --name "$APP" --resource-group "$RG" --query defaultHostname -o tsv)"
