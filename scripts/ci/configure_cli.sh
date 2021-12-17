#!/bin/bash
echo "PIP3 version $(pip3 --version)"

pip3 install linode-cli --upgrade
# install boto dependency for the obj CLI plugin
# required to upload artifacts to Object Storage  
pip3 install boto

# write linode-cli config
mkdir -p ~/.config
touch ~/.config/linode-cli

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
