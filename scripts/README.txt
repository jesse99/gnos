This directory contains miscellaneous scripts used during the operation of gnos:
* sparkline.R is a mustache template used to create a sparkline for sample data (e.g. interface stats).
* snmp-modeler.py is a gnos modeler which uses snmp to gather detailed information about a device.
* ssh-modeler.py is a gnos modeler which uses ssh and standard Linux commands to gather summary information about a device.
* base_modeler.py contains helpers used by Python modelers.

The json files are used by the modelers to discover which devices need to be modeled for a particular network. There are a number of top-level entries:
* network - is the name of the network. It's typically used by clients within window titles.
* poll-rate - is the interval in seconds at which modelers should probe devices.
* client - is the IP address of the machine which should run the modelers. 
* path - is the path component of the URL modelers should use when PUTing.
* admin_network - is set when multiple devices share a network admin ip address. (Setting this prevents gnos from thinking that every device part of the admin network is one hop away from every other device).

Each device in the network should also be listed. Devices have the following required entries:
* <key> - The device entries are keyed using their name. This is the name used by clients in the main view.
* ip - The administrative IP of the device, i.e. the IP the modeler uses to probe the device.
* modeler - Full name of the script used to probe the device.

Devices also have modeler specific entries. For snmp these are:
* links - List of device names used to enumerate edges between them. Note that the edge only has to be specified in one direction.
* authentication - pasted directly into the snmpbulkwalk command line. For snmp v2 this will be something like "-v2c -c public". For authenticated v3 it will be something like "-v3 -u net_user -l authPriv -a md5 -A authpass -x des -X privpass".
* mibs - space separated list of MIBs to query. These must be MIBs that snmp.py knows how to process. TODO: mention which these are.

For linux_ssh these are:
* ssh - The ssh command line used to access the device. If the client machine is able to ssh into the device without a password something like "ssh root@" can be used. Otherwise sshpass can be used: "sshpass -p root ssh -f root@".
