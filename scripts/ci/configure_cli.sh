#!/bin/bash
apt-get update
apt-get -y install python3-venv python3-pip
pip3 install linode-cli --upgrade

# write linode-cli config
cat << EOF >> ~/.config/linode-cli
[DEFAULT]
default-user = alexpikalov

[alexpikalov]
token = $LINODE_API_TOKEN
region = eu-central
type = g6-nanode-1
image = linode/debian9-kube-v1.22.2
EOF

linode-cli show-users
