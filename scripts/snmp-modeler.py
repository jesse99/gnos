#!/usr/bin/python
# This script uses snmp to periodically model a network, encodes it into a json 
# dictionary, and ships the dictionary off to gnos using an http POST.  This
# is designed to be a generic modeler suitable for pretty much any device 
# running SNMP. Other modelers can be used to model more specialized
# functionality (like OSPF and PIM).
#
# We use a Python script instead of simply doing this within gnos for a few
# different reasons:
# 1) There are already Python snmp wrapper libraries. (This was written when
# the code still used pysnmp. Unfortunately pysnmp is not documented very well
# once you start doing anything sophisticated. It also has a rather ridiculous API).
# 2) Using a separate script will make it easier for gnos to manage multiple LANs.
# 3) This separation simplifies development. In particular gnos can run on a 
# developer machine and the script can run on an arbitrary machine connected
# to an arbitrary LAN.
# 4) This design makes it easy for users to write custom modelers using ssh
# or whatever.
import json, itertools, httplib, logging, logging.handlers, re, socket, subprocess, sys, threading, time

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)

logger = logging.getLogger('snmp-modeler')
connection = None

# http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=sysDescr&translate=Translate&submitValue=SUBMIT&submitClicked=true
# SNMPv2-MIB::sysDescr.0 Linux RTR-4 2.6.39.4 #1 Fri Apr 27 02:41:53 PDT 2012 i686
# SNMPv2-MIB::sysObjectID.0 NET-SNMP-MIB::netSnmpAgentOIDs.10
# DISMAN-EVENT-MIB::sysUpTimeInstance 397724214
# SNMPv2-MIB::sysContact.0 support@blargh.com
# SNMPv2-MIB::sysName.0 RTR
# SNMPv2-MIB::sysLocation.0 closet
# SNMPv2-MIB::sysORLastChange.0 1
# SNMPv2-MIB::sysORID[1] IP-MIB::ip
# ...
# SNMPv2-MIB::sysORDescr[1] The MIB module for managing IP and ICMP implementations
# ...
# SNMPv2-MIB::sysORUpTime[1] 0
# ...
def process_system(ip, data, contents):
	#dump_snmp(ip, 'system', contents)
	target = 'entities:%s' % ip
	key = 'alpha'		# want these to appear before most other labels
	up_time = get_value(contents, "%s", 'sysUpTime')
	if not up_time:
		up_time = get_value(contents, "%s", 'sysUpTimeInstance')
	if up_time:
		up_time = float(up_time)/100.0
		add_label(data, target, 'uptime: %s' % secs_to_str(up_time), key, level = 2, style = 'font-size:small')
		if up_time < 60.0:
			# TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
			open_alert(data, target, key = 'uptime', mesg = 'Device rebooted.', resolution = '', kind = 'error')
		else:
			close_alert(data, target, key = 'uptime')
		
	add_label(data, target, get_value(contents, '%s', 'sysDescr'), key, level = 3, style = 'font-size:x-small')
	
	add_label(data, target, get_value(contents, "%s", 'sysContact'), key, level = 4, style = 'font-size:x-small')
	add_label(data, target, get_value(contents, "located in %s", 'sysLocation'), key, level = 4, style = 'font-size:x-small')
	
# A lot of these stats are deprecated in favor of entries in ipSystemStatsTable. But that isn't always available.
# IP-MIB::ipForwarding.0 forwarding
# IP-MIB::ipDefaultTTL.0 64
# IP-MIB::ipInReceives.0 26551558
# IP-MIB::ipInHdrErrors.0 0
# IP-MIB::ipInAddrErrors.0 0
# IP-MIB::ipForwDatagrams.0 0
# IP-MIB::ipInUnknownProtos.0 0
# IP-MIB::ipInDiscards.0 0
# IP-MIB::ipInDelivers.0 26550457
# IP-MIB::ipOutRequests.0 25867018
# IP-MIB::ipOutDiscards.0 0
# IP-MIB::ipOutNoRoutes.0 0
# IP-MIB::ipReasmTimeout.0 0
# IP-MIB::ipReasmReqds.0 0
# IP-MIB::ipReasmOKs.0 0
# IP-MIB::ipReasmFails.0 0
# IP-MIB::ipFragOKs.0 0
# IP-MIB::ipFragFails.0 0
# IP-MIB::ipFragCreates.0 0
# IP-MIB::ipRoutingDiscards.0 0
# IP-MIB::ipAdEntAddr[10.0.4.2] 10.0.4.2
# ...
# IP-MIB::ipAdEntIfIndex[10.0.4.2] 7
# ...
# IP-MIB::ipAdEntNetMask[10.0.4.2] 255.255.255.0
# ...
# IP-MIB::ipAdEntBcastAddr[10.0.4.2] 1
# ...
# RFC1213-MIB::ipRouteDest[10.0.4.0] 10.0.4.0
# ...
# RFC1213-MIB::ipRouteIfIndex[10.0.4.0] 7
# ...
# RFC1213-MIB::ipRouteMetric1[10.0.4.0] 0
# ...
# RFC1213-MIB::ipRouteNextHop[10.0.4.0] 0.0.0.0
# ...
# RFC1213-MIB::ipRouteType[10.0.4.0] direct
# ...
# RFC1213-MIB::ipRouteProto[10.0.4.0] local
# ...
# RFC1213-MIB::ipRouteMask[10.0.4.0] 255.255.255.0
# ...
# RFC1213-MIB::ipRouteInfo[10.0.4.0] SNMPv2-SMI::zeroDotZero
# ...
# IP-MIB::ipNetToMediaIfIndex[5][10.104.0.254] 5
# ...
# IP-MIB::ipNetToMediaPhysAddress[5][10.104.0.254] 0:19:bb:5f:59:8a
# ...
# IP-MIB::ipNetToMediaNetAddress[5][10.104.0.254] 10.104.0.254
# ...
# IP-MIB::ipNetToMediaType[5][10.104.0.254] dynamic
# ...
def process_ip(ip, data, contents):
	#dump_snmp(ip, 'ip', contents)
	target = 'entities:%s' % ip
	key = 'zeppo'
	add_label(data, target, get_value(contents, '%s', 'ipForwarding'), key, level = 5, style = 'font-size:x-small')
	
def add_label(data, target, label, key, level = 0, style = ''):
	if label:
		sort_key = '%s-%s' % (level, key)
		if style:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key, 'style': style})
		else:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key})
		
def open_alert(data, target, key, mesg, resolution, kind):
	data['alerts'].append({'entity-id': target, 'key': key, 'mesg': mesg, 'resolution': resolution, 'kind': kind})

def close_alert(data, target, key):
	data['alerts'].append({'entity-id': target, 'key': key})

# In general lines look like:
#    IP-MIB::icmpOutErrors.0 0
#    TCP-MIB::tcpConnLocalAddress[127.0.0.1][2601][0.0.0.0][0] 127.0.0.1
# where the stuff in brackets is optional.
def get_value(contents, fmt, name):
	# Not clear whether it's faster to split the lines and match as we iterate
	# or to use regexen. But splitting large amounts of text is rather slow 
	# and often relatively little of the MIB is used.
	expr = re.compile(r'::%s (?= \W) .*? \ (.+)$' % name, re.MULTILINE | re.VERBOSE)	# TODO: faster to cache these
	match = re.search(expr, contents)
	if match:
		return fmt % match.group(1)
	return None

def dump_snmp(ip, name, contents):
	logger.debug('%s %s:' % (ip, name))
	logger.debug('%s' % (contents))
		
def secs_to_str(secs):
	if secs >= 365.25*86400:
		return '%.2f years' % (secs/(365.25*86400))		# http://en.wikipedia.org/wiki/Month#Month_lengths
	elif secs >= 365.25*86400/12:
		return '%.2f months' % (secs/(365.25*86400/12))
	elif secs >= 86400:
		return '%.1f days' % (secs/(86400))
	elif secs >= 60*60:
		return '%.1f hours' % (secs/(60*60))
	elif secs >= 60:
		return '%.0f minutes' % (secs/(60))
	elif secs >= 1:
		return '%.0f seconds' % secs
	else:
		return '%.3f msecs' % (1000*secs)

def send_update(config, data):
	logger.debug("sending update")
	logger.debug("%s" % json.dumps(data, sort_keys = True, indent = 4))
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
	send_update(config, {"modeler": "config", "entities": entities})

def run_process(command):
	process = subprocess.Popen(command, bufsize = 8*1024, shell = True, stdout = subprocess.PIPE, stderr = subprocess.PIPE)
	(outData, errData) = process.communicate()
	if process.returncode != 0:
		logger.error(errData)
		raise ValueError('return code was %s:' % process.returncode)
	return outData

class DeviceThread(threading.Thread):
	def __init__(self, ip, community, mib_names):
		threading.Thread.__init__(self)
		self.ip = ip
		self.__community = community
		self.__mib_names = mib_names
		self.results = None									# mapping from mib name to results of the query for that mib
		
	def run(self):
		self.results = {}
		for name in self.__mib_names:
			self.results[name] = self.__walk_mib(name)
		
	# When only a few items are used it would be faster to use something like:
	# snmpbulkget -v2c -c public 10.101.0.2 -Oq -Ot -OU -OX ipRouteMask ipFragFails ipDefaultTTL
	def __walk_mib(self, name):
		command = 'snmpbulkwalk -v2c -c "%s" %s -Oq -Ot -OU -OX %s' % (self.__community, self.ip, name)
		try:
			result = run_process(command)
		except:
			logger.error("Error executing `%s`" % command, exc_info = True)
			result = ''
		return result

class Poll(object):
	def __init__(self, args, config):
		self.__args = args
		self.__config = config
		self.__startTime = time.time()
		self.__handlers = {'system': process_system, 'ip': process_ip}
	
	def run(self):
		rate = self.__config['poll-rate']
		while time.time() - self.__startTime < self.__args.duration:
			currentTime = time.time()
			if not self.__args.put:
				logger.info("-" * 60)
				
			threads = self.__spawn_threads()
			data = self.__process_threads(threads)
			send_update(self.__config, data)
			
			elapsed = time.time() - currentTime
			logger.info('elapsed: %.1f seconds' % elapsed)
			time.sleep(max(rate - elapsed, 5))
			
	# Devices can have significant variation in how quickly they respond to SNMP queries
	# so simply joining them one after another isn't great, but it's simple and should work
	# fine most of the time.
	def __process_threads(self, threads):
		data = {'modeler': 'snmp', 'entities': [], 'relations': [], 'labels': [], 'gauges': [], 'details': [], 'alerts': []}
		for thread in threads:
			thread.join(3.0)
			
			target = 'entities:%s' % thread.ip
			if not thread.isAlive():
				close_alert(data, target, key = 'device down')
				for (mib, contents) in thread.results.items():
					self.__handlers[mib](thread.ip, data, contents)
			else:
				open_alert(data, target, key = 'device down', mesg = 'Device is down.', resolution = 'Check the power cable, power it on if it is off, check the IP address, verify routing.', kind = 'error')
		return data
	
	# This could be a lot of threads but they spend nearly all their time blocked so
	# that should be OK.
	def __spawn_threads(self):
		threads = []
		for (name, device) in self.__config["devices"].items():
			thread = DeviceThread(device['ip'], device['community'], self.__handlers.keys())
			thread.start()
			threads.append(thread)
		return threads

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
	poller = Poll(args, config)
	poller.run()
finally:
	if connection:
		connection.close()
