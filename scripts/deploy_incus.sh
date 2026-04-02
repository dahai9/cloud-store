#!/bin/bash
set -e

# Configurable variables (can be overridden by environment variables)
TRUST_PASSWORD="${TRUST_PASSWORD:-cloudstore123}"
INCUS_PORT="${INCUS_PORT:-18443}"
STORAGE_POOL_NAME="default"
NETWORK_NAME="incusbr0"

echo "==== Incus Node Deployment Script ===="
echo "Port: $INCUS_PORT"
echo "Trust Password: [HIDDEN]"

# 1. OS Detection
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VER=$VERSION_ID
else
    echo "Unsupported OS"
    exit 1
fi

echo "Detected OS: $OS $VER"

# 2. Repository Setup and Installation
case "$OS" in
    ubuntu|debian)
        echo "Setting up Zabbly repository for $OS..."
        apt-get update && apt-get install -y curl gnupg2
        mkdir -p /etc/apt/keyrings/
        curl -fsSL https://pkgs.zabbly.com/key.asc | gpg --dearmor -o /etc/apt/keyrings/zabbly.gpg
        
        # Add repo (using the recommended codename logic)
        CODENAME=$(lsb_release -sc 2>/dev/null || echo $VERSION_CODENAME)
        echo "deb [signed-by=/etc/apt/keyrings/zabbly.gpg] https://pkgs.zabbly.com/incus/stable $CODENAME main" > /etc/apt/sources.list.d/zabbly-incus.list
        
        apt-get update
        apt-get install -y incus
        ;;
    almalinux|rocky|rhel|centos)
        echo "Setting up COPR repository for $OS..."
        dnf install -y 'dnf-command(copr)'
        dnf copr enable -y gabe/incus
        dnf install -y incus
        ;;
    *)
        echo "OS $OS is not explicitly supported by this automated script."
        exit 1
        ;;
esac

# 3. Kernel module check (essential for LXC/Incus)
modprobe vhost_vsock || true

# 4. Prepare Preseed configuration
cat <<EOF > /tmp/incus-preseed.yaml
config:
  core.https_address: :$INCUS_PORT
  core.trust_password: $TRUST_PASSWORD
networks:
- config:
    ipv4.address: 10.0.100.1/24
    ipv4.nat: "true"
    ipv6.address: none
  description: "Cloud Store NAT Bridge"
  name: $NETWORK_NAME
  type: bridge
  project: default
storage_pools:
- config:
    source: /var/lib/incus/storage-pools/$STORAGE_POOL_NAME
  description: "Default storage pool"
  name: $STORAGE_POOL_NAME
  driver: dir
  project: default
profiles:
- config: {}
  description: "Default Incus profile"
  devices:
    eth0:
      name: eth0
      network: $NETWORK_NAME
      type: nic
    root:
      path: /
      pool: $STORAGE_POOL_NAME
      type: disk
  name: default
  project: default
projects:
- config:
    features.networks: "true"
    features.profiles: "true"
    features.images: "true"
    features.storage.volumes: "true"
  description: "Default project"
  name: default
EOF

# 5. Initialize Incus
echo "Initializing Incus with preseed..."
# If already initialized, this might fail or do nothing depending on version. 
# We use --force if supported or just pipe to init.
incus admin init --preseed < /tmp/incus-preseed.yaml

echo "==== Deployment Successful ===="
echo "Incus is listening on port $INCUS_PORT"
echo "You can now add this node to Cloud Store using this IP and the configured trust password."
