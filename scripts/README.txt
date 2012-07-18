The json file is used to configure both the snmp-modeler.py script and the server itself. The json file must be a dictionary and must include the following keys:

poll-rate: internal in seconds that snmp-modeler.py uses for snmp queries.

client: ip address of the machine that should run the snmp-modeler.py script. For now the server must be able to ssh into this device without a password.

server: ip address the server binds to. This is the address that snmp-modeler.py will send PUTs to with the results of the snmp queries. Note that if the server is invoked with --admin the server will also bind to localhost.

port: The port that the server should bind to.

path: The path component of the URL that snmp-modeler.py uses when PUting results.

devices: Ip addresses and snmp community strings that snmp-modeler.py queries.
