#!/usr/bin/python
# This script uses snmp to periodically model a network, encodes it into a json 
# dictionary, and ships the dictionary off to gnos using an http POST. 
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
import json, itertools, httplib, logging, logging.handlers, sys, time

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

# Does an snmpwalk sort of thing to query IF-MIB, IP-MIB, and SNMPv2-MIB and 
# return the results as a dict suitable for shipping off to gnos as json data. Note
# that the values are all strings because snmp often uses unsigned 32-bit numbers
# (e.g. for Counter32) which may not fit into a Python int on 32-bit Pythons.
class QueryDevice(object):
	def __init__(self, name, ip, community):
		self.__adminName = name
		self.__adminIp = ip
		self.__community = community
		self.__generator = cmdgen.CommandGenerator()	# http://pysnmp.sourceforge.net/quickstart.html
		
		self.__ifSymbols = ['ifDescr', 'ifType', 'ifMtu', 'ifPhysAddress', 'ifAdminStatus', 'ifOperStatus', 'ifLastChange', 'ifSpeed', 'ifOutQLen', 'ifInOctets', 'ifInUcastPkts', 'ifInNUcastPkts', 'ifInDiscards', 'ifInErrors', 'ifInUnknownProtos', 'ifOutOctets', 'ifOutUcastPkts', 'ifOutNUcastPkts', 'ifOutDiscards', 'ifOutErrors']
		self.__ipSymbols = ['ipForwarding', 'ipDefaultTTL', 'ipNetToMediaType', 'ipInReceives', 'ipInHdrErrors', 'ipInAddrErrors', 'ipForwDatagrams', 'ipInUnknownProtos', 'ipInDiscards', 'ipInDelivers', 'ipOutRequests', 'ipOutDiscards', 'ipOutNoRoutes', 'ipReasmReqds', 'ipReasmOKs', 'ipReasmFails', 'ipFragOKs', 'ipFragFails', 'ipFragCreates']
		self.__sysSymbols = ['sysDescr', 'sysUpTime', 'sysContact', 'sysName', 'sysLocation']
		
	def run(self):
		result = {}			# {'ip-forwarding': true, 'ip-stat-N': '100', 'interfaces': [interface dict]} (stats are str because Python has no unigned int type)
		interfaces = {}		# snmp index => interface dict
		self.__ignored = set()
		self.__errors = []
		
		# Walk the entire IF-MIB (this will give us all the interface related
		# information with the notable exception of the ip address).
		rows = self.__walkIF()
		if type(rows) == str:
			self.__errors.append(rows)
		else:
			for (symbol, index, value) in rows:
				interface = interfaces.setdefault(index, {})
				self.__populateInterface(interface, symbol, value)
				
		# Walk the entire IP-MIB. Currently we don't use everything (e.g. routing
		# but in the future we should use most of it).
		rows = self.__walkIP()
		if type(rows) == str:
			self.__errors.append(rows)
		else:
			extras = []									# [[ifIndex, ip-address, net-mask], ...]
			for (symbol, index, value) in rows:
				if symbol == 'ipAdEntIfIndex':		# index == ip-address, value == if-index
					m = list(itertools.ifilter(lambda x: x[0] == value or x[1] == index, extras))
					if m:
						m[0][0] = value
						m[0][1] = index
					else:
						extras.append([value, index, None])
				elif symbol == 'ipAdEntNetMask':	# index == ip-address, value == net-mask
					m = list(itertools.ifilter(lambda x: x[1] == index, extras))
					if m:
						m[0][2] = value
					else:
						extras.append([None, index, value])
				else:
					self.__populateResult(result, symbol, value, self.__ipSymbols)
			
			# Update interfaces with info from IP-MIB.
			for extra in extras:
				interface = interfaces.get(extra[0], {})
				interface['ipAdEntAddr'] = extra[1]
				interface['ipAdEntNetMask'] = extra[2]
			
		# Update result with info from SNMPv2-MIB.
		rows = self.__querySystem()
		if type(rows) == str:
			self.__errors.append(rows)
		else:
			for (symbol, index, value) in rows:
				self.__populateResult(result, symbol, value, self.__sysSymbols)
			
		self.__dontCare()
		result['interfaces'] = [i for i in interfaces.values() if len(i) > 0]
		return result
		
	@property
	def adminName(self):
		return self.__adminName
		
	@property
	def adminIp(self):
		return self.__adminIp
		
	@property
	def error(self):
		return '\n'.join(self.__errors)
		
	@property
	def ignores(self):
		return self.__ignored
		
	def __dontCare(self):
		self.__ignored.discard('ifIndex')							# this is used
		self.__ignored.discard('ifNumber')						# we can figure out how many interfaces are present
		self.__ignored.discard('ifSpecific')						# reference to the particular media being used to realize the interface
		
		self.__ignored.discard('ipAdEntIfIndex')					# this is used
		self.__ignored.discard('ipAdEntNetMask')				# this is used
		self.__ignored.discard('ipNetToMediaIfIndex')			# same as ipAdEntIfIndex
		self.__ignored.discard('ipAdEntAddr')					# ip address (which we figure out elsewhere)
		self.__ignored.discard('ipNetToMediaNetAddress')		# ip address (which we figure out elsewhere)
		self.__ignored.discard('ipAdEntBcastAddr')				# value of the least-significant bit in the IP broadcast
		self.__ignored.discard('ipNetToMediaPhysAddress')	# mac address (which we figure out elsewhere)
		
		self.__ignored.discard('sysObjectID')					# these two were not requested (but bulkCmd returns extra symbols)
		self.__ignored.discard('sysORLastChange')
		
	def __populateInterface(self, interface, symbol, value):
		if value:
			if symbol in self.__ifSymbols:
				interface[symbol] = value
			else:
				self.__ignored.add(symbol)
	
	def __populateResult(self, result, symbol, value, symbols):	# http://www.opensource.apple.com/source/net_snmp/net_snmp-9/net-snmp/mibs/IP-MIB.txt
		if value:
			if symbol in symbols:
				result[symbol] = value
			else:
				self.__ignored.add( symbol)
	
	# Returns a sequence like [(symbol, index, value)] where symbol is a name like
	# ifType or ifOperStatus, index is used to match up associated symbols and 
	# value is string dependent on the symbol. On errors a string with the error 
	# message is returned.
	def __walkIF(self):
		result = []
		try:
			errorIndication, errorStatus, errorIndex, table = self.__generator.nextCmd(	# http://www.opensource.apple.com/source/net_snmp/net_snmp-9/net-snmp/mibs/IF-MIB.txt
				cmdgen.CommunityData('gnos-agent', self.__community),
				cmdgen.UdpTransportTarget((self.__adminIp, 161)),
				(('IF-MIB', ''), ))
				
			result = self.__processResult(errorIndication, errorStatus, errorIndex, table)
		except:
			logger.error("Error walking IF-MIB", exc_info = True)
		return result
			
	def __walkIP(self):
		result = []
		try:
			errorIndication, errorStatus, errorIndex, table = self.__generator.nextCmd(
				cmdgen.CommunityData('gnos-agent', self.__community),
				cmdgen.UdpTransportTarget((self.__adminIp, 161)),
				(('IP-MIB', ''), ))
			
			result = self.__processResult(errorIndication, errorStatus, errorIndex, table)
		except:
			logger.error("Error walking IP-MIB", exc_info = True)
		return result
	
	def __querySystem(self):
		result = []
		try:
			errorIndication, errorStatus, errorIndex, rows = self.__generator.getCmd(	# http://downloads01.smarttech.com/media/products/hub/snmpv2-mib.txt
				cmdgen.CommunityData('gnos-agent', self.__community),
				cmdgen.UdpTransportTarget((self.__adminIp, 161)),
				(('SNMPv2-MIB', 'sysDescr'), 0),
				(('SNMPv2-MIB', 'sysUpTime'), 0),
				(('SNMPv2-MIB', 'sysContact'), 0),
				(('SNMPv2-MIB', 'sysName'), 0),
				(('SNMPv2-MIB', 'sysLocation'), 0))
			
			result = self.__processResult(errorIndication, errorStatus, errorIndex, [rows])
		except:
			logger.error("Error getting SNMPv2-MIB", exc_info = True)
		return result
	
	def __processResult(self, errorIndication, errorStatus, errorIndex, table):
		if errorIndication:
			return errorIndication
		elif errorStatus:
			return '%s at %s\n' % (errorStatus.prettyPrint(), errorIndex and table[int(errorIndex)-1] or '?')
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

# TODO: Could substantially speed this up by using a thread pool. If this
# is done we'll want to allow admins to configure the number of threads:
# they know how many cores are on the manager machine and how many
# they wish to dedicate to polling. Moreover, for networks used as test networks,
# testers would like to keep the network load to a minimum.
class Poll(object):
	def __init__(self, args, config):
		self.__args = args
		self.__config = config
		self.__startTime = time.time()
		self.__queries = [QueryDevice(name, device["ip"], device["community"]) for name, device in config["devices"].items()]
		if self.__args.put:
			self.__connection = httplib.HTTPConnection(self.__config['server'], strict = True, timeout = 10)
		else:
			self.__connection = None
	
	def run(self):
		rate = self.__config['poll-rate']
		while time.time() - self.__startTime < self.__args.duration:
			currentTime = time.time()
			if not self.__args.put:
				logger.info("-" * 60)
				
			state = {}
			for query in self.__queries:
				result = query.run()
				if self.__args.put:
					state[query.adminIp] = result
				else:
					self.__print(query, result)
				
			if logger.isEnabledFor(logging.DEBUG):
				logger.debug('State:\n%s' % json.dumps(state, sort_keys = True, indent = 4))
			if self.__args.put:
				self.__put(state)
			elapsed = time.time() - currentTime
			logger.info('elapsed: %.1f seconds' % elapsed)
			time.sleep(max(rate - elapsed, 5))
			
		if self.__connection:
			self.__connection.close()
	
	# If there is an error PUTing then exit. There are a number of reasons we can get errors:
	# 1) The server can be down in which case exiting is appropiate.
	# 2) There could be a transient networking error. Exiting is OK because the server will restart us.
	# 3) There could be a prolonged network failure. Exiting is OK because we can't communicate with the server.
	# 4) There could be a bug in this script or in the server. Exiting may or may not be appropriate but
	# at least this way the server (and hopefully the admin) will realize there is a serious problem. 
	def __put(self, state):
		try:
			body = json.dumps(state)
			headers = {"Content-type": "application/json", "Accept": "text/html"}
			
			self.__connection.request("PUT", self.__config['path'], body, headers)
			response = self.__connection.getresponse()
			if not str(response.status).startswith('2'):
				logger.error("Error PUTing: %s %s" % (response.status, response.reason))
				sys.exit(3)
		except Exception as e:
			logger.error("Error PUTing to %s:%s: %s" % (self.__config['server'], self.__config['path'], e), exc_info = False)
			#logger.error("Error PUTing to %s:%s" % (self.__config['server'], self.__config['path']), exc_info = True)	# TODO: should do these instead
			#sys.exit(3)	
				
	def __print(self, query, result):
		logger.info("%s:" % query.adminName)
		if self.__args.verbose <= 2:
			for i in result['interfaces']:
				status = i.get('ifOperStatus', 'status?')
				if status.startswith("up"):
					status = ''
				elif status.startswith('down'):
					status = 'DOWN'
				logger.info("   %s %s %s" % (i.get('ifDescr', 'eth?'), i.get('ipAdEntAddr', ''), status))
		else:
			entries = [item for item in result.items() if item[0] != 'interfaces']
			entries.sort(lambda x, y: cmp(x[0], y[0]))
			for (k, v) in entries:
				logger.debug('   %s: %s' % (k, v))
					
			for i in result['interfaces']:
				logger.debug('   {')
				entries = [item for item in i.items()]
				entries.sort(lambda x, y: cmp(x[0], y[0]))
				for (k, v) in entries:
					logger.debug('      %s: %s' % (k, v))
				logger.debug('   }')
				
		if query.ignores:
			logger.info("   ignored: %s" % ', '.join(query.ignores))
		if query.error:
			logger.error('   errors: %s' % query.error)

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

# Start polling each device.
poller = Poll(args, config)
poller.run()
