#!/usr/bin/python
# This script uses snmp to periodically model a network, encodes it into a json 
# dictionary, and ships the dictionary off to gnos using an http POST.  This
# is designed to be a generic modeler suitable for pretty much any device 
# running SNMP. Other modelers can be used to model more specialized
# functionality (like OSPF and PIM).
#
# We use a Python script instead of simply doing this within gnos for a few
# different reasons:
# 1) There are already Python snmp wrapper libraries.
# 2) Using a separate script will make it easier for gnos to manage multiple LANs.
# 3) This separation simplifies development. In particular gnos can run on a 
# developer machine and the script can run on an arbitrary machine connected
# to an arbitrary LAN.
# 4) This design makes it easy for users to write custom modelers using ssh
# or whatever.
import json, itertools, httplib, logging, logging.handlers, socket, sys, time

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)

# TODO: not sure using pysnmp is the best way to go:
# 1) The documentation is truly horrible once you go beyond very basic usage.
# 2) There are way too many hoops you have to jump through for anything non-trivial.
try:
	from pysnmp.entity.rfc3413.oneliner import cmdgen
except:
	sys.stderr.write("pysnmp is missing: install python-pysnmp4\n")
	sys.exit(2)
	
logger = logging.getLogger('snmp-modeler')
connection = None

def send_update(config, data):
	logger.debug("sending update")
	if connection:
		try:
			body = json.dumps(data)
			headers = {"Content-type": "application/json", "Accept": "text/html"}
			
			connection.request("PUT", config['path'], body, headers)
			response = connection.getresponse()
			response.read()			# we don't use this but we must call it (or, on the second call, we'll get ResponseNotReady errors)
			if not str(response.status).startswith('2'):
				logger.error("Error PUTing: %s %s" % (response.status, response.reason))
				raise Exception("PUT failed")
		except Exception as e:
			address = "%s:%s" % (config['server'], config['port'])
			logger.error("Error PUTing to %s:%s: %s" % (address, config['path'], e), exc_info = type(e) != socket.error)
			raise Exception("PUT failed")

def send_entities(config):
	entities = []
	for (name, device) in config["devices"].items():
		if device['type'] == "router":
			style = "font-size:larger font-weight:bolder"
		else:
			style = ""
		entity = {"id": device['ip'], "label": name, "style": style}
		logger.debug("entity: %s" % entity)
		entities.append(entity)
	send_update(config, {"source": "config", "entities": entities})

# Parse command line.
parser = argparse.ArgumentParser(description = "Uses snmp to model a network and sends the result to a gnos server.")
parser.add_argument("--dont-put", dest = 'put', action='store_false', default=True, help = 'log results instead of PUTing them')
parser.add_argument("--duration", action='store', default=float('inf'), type=float, metavar='SECS', help = 'amount of time to poll (for testing)')
parser.add_argument("--stdout", action='store_true', default=False, help = 'log to stdout instead of snmp-modeler.log')
parser.add_argument("--verbose", "-v", action='count', help = 'print extra information')
parser.add_argument("--version", "-V", action='version', version='%(prog)s 0.1')	# TODO: keep this version synced up with the gnos version
parser.add_argument("config", metavar = "CONFIG-FILE", help = "path to json formatted configuration file")
args = parser.parse_args()

# Configure logging.
if args.verbose <= 1:
	logger.setLevel(logging.WARNING)
elif args.verbose == 2:
	logger.setLevel(logging.INFO)
else:
	logger.setLevel(logging.DEBUG)
	
if args.stdout:
	handler = logging.StreamHandler()
	formatter = logging.Formatter('%(asctime)s  %(message)s', datefmt = '%I:%M:%S')
else:
	# Note that we don't use SysLogHandler because, on Ubuntu at least, /etc/default/syslogd
	# has to be configured to accept remote logging requests.
	handler = logging.FileHandler('snmp-modeler.log', mode = 'w')
	formatter = logging.Formatter('%(asctime)s  %(message)s', datefmt = '%m/%d %I:%M:%S %p')
handler.setFormatter(formatter)
logger.addHandler(handler)

# Read config info.
config = None
with open(args.config, 'r') as f:
	config = json.load(f)
	
if args.put:
	address = "%s:%s" % (config['server'], config['port'])
	connection = httplib.HTTPConnection(address, strict = True, timeout = 10)

try:
	# Send entity information to the server.
	send_entities(config)
	
	# Start polling each device.
	#poller = Poll(args, config)
	#poller.run()
finally:
	if connection:
		connection.close()
