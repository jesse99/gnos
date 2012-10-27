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
import json, itertools, httplib, logging, logging.handlers, socket, sys, threading, time

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

# http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?objectInput=sysDescr&translate=Translate&submitValue=SUBMIT&submitClicked=true
# sysDescr  Linux RTR4 2.6.39.4 #1 Fri Apr 27 02:41:53 PDT 2012 i686
# sysObjectID  1.3.6.1.4.1.8072.3.2.10
# sysUpTime  389960053					The time (in hundredths of a second) since the network management portion of the system was last re-initialized.
# sysContact  support@blargh.com
# sysName  RTR
# sysLocation  closet
# sysORLastChange  1					The value of sysUpTime at the time of the most recent change in state or value of any instance of sysORID (within sysORTable)
# sysORTable								The (conceptual) table listing the capabilities of the local SNMP application acting as a command responder with respect to various MIB modules
#    ...
def process_snmpv2(ip, data, contents):
	#dump_snmp(ip, 'SNMPv2-MIB', contents)
	key = 'alpha'		# want these to appear before most other labels
	up_time = get_value(contents, "%s", 'sysUpTime')
	if up_time:
		up_time = float(up_time)/100.0
		add_label(data, ip, 'uptime: %s' % secs_to_str(up_time), key, level = 2)
		if up_time < 60.0:
			open_alert(data, ip, key = 'uptime', mesg = 'Device rebooted.', resolution = '', style = 'alert-type:error')
		else:
			close_alert(data, ip, key = 'uptime')
		
	add_label(data, ip, get_value(contents, 'description: %s', 'sysDescr'), key, level = 3)
	
	add_label(data, ip, get_value(contents, "contact: %s", 'sysContact'), key, level = 4)
	add_label(data, ip, get_value(contents, "location: %s", 'sysLocation'), key, level = 4)
	
def add_label(data, target, label, key, level = 0, style = ''):
	if label:
		sort_key = '%s-%s' % (level, key)
		if style:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key, 'style': style})
		else:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key})
		
def open_alert(data, target, key, mesg, resolution, style):
	data['alerts'].append({'entity-id': target, 'key': key, 'mesg': mesg, 'resolution': resolution, 'style': style})

def close_alert(data, target, key):
	data['alerts'].append({'entity-id': target, 'key': key})

def get_value(contents, fmt, name):
	# Kind of lame to do a linear search but symbol isn't unique.
	for (symbol, index, value) in contents:
		if symbol == name:
			return fmt % value
	return None

def dump_snmp(ip, name, contents):
	logger.debug('%s %s:' % (ip, name))
	for (symbol, index, value) in contents:
		logger.debug('   %s %s %s' % (symbol, index, value))
		
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
	send_update(config, {"source": "config", "entities": entities})

class DeviceThread(threading.Thread):
	def __init__(self, ip, community, mib_names):
		threading.Thread.__init__(self)
		self.ip = ip
		self.__community = community
		self.__mib_names = mib_names
		self.__generator = cmdgen.CommandGenerator()	# http://pysnmp.sourceforge.net/quickstart.html
		self.results = None					# mapping from mib name to results of the query for that mib
		
	def run(self):
		self.results = {}
		for name in self.__mib_names:
			self.results[name] = self.__walk_mib(name)
		
	def __walk_mib(self, name):
		try:
			errorIndication, errorStatus, errorIndex, table = self.__generator.nextCmd(	# http://www.opensource.apple.com/source/net_snmp/net_snmp-9/net-snmp/mibs/IF-MIB.txt
				cmdgen.CommunityData('gnos-agent', self.__community),
				cmdgen.UdpTransportTarget((self.ip, 161)),
				((name, ''), ))
				
			result = self.__processResult(name, errorIndication, errorStatus, errorIndex, table)
		except:
			logger.error("Error walking %s" % name, exc_info = True)
			result = []
		return result
		
	def __processResult(self, name, errorIndication, errorStatus, errorIndex, table):
		if errorIndication:
			logger.error("Error processing %s for %s: %s" % (name, self.ip, errorIndication))
			return []
		elif errorStatus:
			logger.error("Error processing %s for %s: %s at %s" % (name, self.ip, errorStatus.prettyPrint(), errorIndex and table[int(errorIndex)-1] or '?'))
			return []
		else:
			result = []
			for row in table:
				for oid, val in row:
					try:
						(symbol, module), index = self.__oidToMibName(self.__generator.mibViewController, oid)		# module will be 'IF-MIB'
						value = cmdgen.mibvar.cloneFromMibValue(self.__generator.mibViewController, module, symbol, val)	# for stuff like ifOperStatus val will be a number and value will be something human readable
						if value:
							result.append((symbol, index, value.prettyPrint()))
						else:
							result.append((symbol, index, ''))
					except:
						logger.error("Error proccessing oid %s" % (".".join([str(o) for o in oid])), exc_info = True)
			return result
	
	# This is similar to the oidToMibName function in pysnmp except that instead of raising
	# an exception if the entire oid cannot be converted it returns the unconverted bits in
	# the last element. This is neccesary because there are OIDs that we care about (such
	# as ipAdEntAddr) where the last values are actually components of an ip address.
	def __oidToMibName(self, mibView, oid):
		_oid, label, suffix = mibView.getNodeNameByOid(tuple(oid))
		modName, symName, __suffix = mibView.getNodeLocation(_oid)
		mibNode, = mibView.mibBuilder.importSymbols(modName, symName)
		if not suffix:
			return (symName, modName), '.'.join(map(lambda v: v.prettyPrint(), suffix))
		elif suffix == (0,): # scalar
			return (symName, modName), ''
		else:
			return (symName, modName), '.'.join([str(v) for v in suffix])

class Poll(object):
	def __init__(self, args, config):
		self.__args = args
		self.__config = config
		self.__startTime = time.time()
	
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
		data = {'source': 'snmp', 'entities': [], 'relations': [], 'labels': [], 'gauges': [], 'details': [], 'alerts': []}
		handlers = {'SNMPv2-MIB': process_snmpv2}
		for thread in threads:
			thread.join(3.0)
			if not thread.isAlive():
				close_alert(data, thread.ip, key = 'device down')
				for (mib, contents) in thread.results.items():
					handlers[mib](thread.ip, data, contents)
			else:
				open_alert(data, thread.ip, key = 'device down', mesg = 'Device is down.', resolution = 'Check the power cable, power it on if it is off, check the IP address, verify routing.', style = 'alert-type:error')
		return data
	
	# This could be a lot of threads but they spend nearly all their time blocked so
	# that should be OK.
	def __spawn_threads(self):
		threads = []
		for (name, device) in self.__config["devices"].items():
			thread = DeviceThread(device['ip'], device['community'], ['SNMPv2-MIB'])
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
