#!/bin/bash
# Configuration
NODES_FILE="scripts/nodes.txt"
DEPLOY_SCRIPT="scripts/deploy_incus.sh"
SSH_USER="root"
INCUS_PORT="18443"

if [ ! -f "$NODES_FILE" ]; then
    echo "Error: $NODES_FILE not found. Please create it with one IP per line."
    exit 1
fi

echo "Starting cluster-wide Incus deployment..."

while IFS= read -r IP || [ -n "$IP" ]; do
    [[ -z "$IP" || "$IP" =~ ^# ]] && continue
    
    echo "----------------------------------------------------"
    echo "Processing Node: $IP"
    echo "----------------------------------------------------"
    
    # Copy the script to the remote node
    scp "$DEPLOY_SCRIPT" "$SSH_USER@$IP:/tmp/deploy_incus.sh"
    
    # Execute the script with environment variables
    ssh "$SSH_USER@$IP" "INCUS_PORT='$INCUS_PORT' bash /tmp/deploy_incus.sh"
    
    echo "Node $IP deployment completed."
done < "$NODES_FILE"

echo "==== All nodes processed ===="
