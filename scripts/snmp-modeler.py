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
import cgi, json, itertools, httplib, logging, logging.handlers, re, socket, subprocess, sys, threading, time

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
#
# SNMPv2-MIB::sysORID[1] IP-MIB::ip		will be one of these for each MIB supported by the device
# SNMPv2-MIB::sysORDescr[1] The MIB module for managing IP and ICMP implementations
# SNMPv2-MIB::sysORUpTime[1] 0
def process_system(ip, data, contents, context):
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
#
# IP-MIB::ipAdEntAddr[10.0.4.2] 10.0.4.2			will be one of these for each interface
# IP-MIB::ipAdEntIfIndex[10.0.4.2] 7
# IP-MIB::ipAdEntNetMask[10.0.4.2] 255.255.255.0
# IP-MIB::ipAdEntBcastAddr[10.0.4.2] 1
# IP-MIB::ipNetToMediaIfIndex[5][10.104.0.254] 5
# IP-MIB::ipNetToMediaPhysAddress[5][10.104.0.254] 0:19:bb:5f:59:8a
# IP-MIB::ipNetToMediaNetAddress[5][10.104.0.254] 10.104.0.254
# IP-MIB::ipNetToMediaType[5][10.104.0.254] dynamic
#
# RFC1213-MIB::ipRouteDest[10.0.4.0] 10.0.4.0	will be one of these for each route
# RFC1213-MIB::ipRouteIfIndex[10.0.4.0] 7
# RFC1213-MIB::ipRouteMetric1[10.0.4.0] 0
# RFC1213-MIB::ipRouteNextHop[10.0.4.0] 0.0.0.0
# RFC1213-MIB::ipRouteType[10.0.4.0] direct
# RFC1213-MIB::ipRouteProto[10.0.4.0] local
# RFC1213-MIB::ipRouteMask[10.0.4.0] 255.255.255.0
# RFC1213-MIB::ipRouteInfo[10.0.4.0] SNMPv2-SMI::zeroDotZero
def process_ip(admin_ip, data, contents, context):
	#dump_snmp(admin_ip, 'ip', contents)
	target = 'entities:%s' % admin_ip
	key = 'zeppo'
	add_label(data, target, get_value(contents, '%s', 'ipForwarding'), key, level = 5, style = 'font-size:x-small')
	
	ips = get_values(contents, "ipAdEntIfIndex")
	for (ip, if_index) in ips.items():
		# create a mapping from device ip to admin ip
		context['ips'][ip] = admin_ip
		
		# create a mapping from if index => device ip
		context['if_indexes'][admin_ip + if_index] = ip
	
	# create a mapping from device ip to network mask
	masks = get_values(contents, "ipAdEntNetMask")
	for (ip, mask) in masks.items():
		context['netmasks'][ip] = mask
	
	# create a table for routing (we can't build relations until we finish building the device to admin ip mapping)
	nexts = get_values(contents, "ipRouteNextHop")
	metrics = get_values(contents, "ipRouteMetric1")
	protocols = get_values(contents, "ipRouteProto")	
	for dest_ip in nexts.keys():
		entry = {'src': admin_ip, 'next hop': nexts.get(dest_ip, ''), 'dest': dest_ip, 'metric': metrics.get(dest_ip, ''), 'protocol': protocols.get(dest_ip, '')}
		context['routes'].append(entry)
	
# IF-MIB::ifNumber.0 13				will be one of these for each interface
# IF-MIB::ifDescr[1] lo			
# IF-MIB::ifType[1] softwareLoopback or tunnel or ethernetCsmacd
# IF-MIB::ifMtu[1] 16436
# IF-MIB::ifSpeed[1] 10000000
# IF-MIB::ifPhysAddress[1] blank or c2:25:a1:a0:30:9b
# IF-MIB::ifAdminStatus[1] up
# IF-MIB::ifOperStatus[1] up
# IF-MIB::ifLastChange[1] 0 or 219192
# IF-MIB::ifInOctets[1] 9840
# IF-MIB::ifInUcastPkts[1] 120
# IF-MIB::ifInNUcastPkts[1] 0 		seems to always be 0
# IF-MIB::ifInDiscards[1] 0			seems to always be 0
# IF-MIB::ifInErrors[1] 0
# IF-MIB::ifInUnknownProtos[1] 0
# IF-MIB::ifOutOctets[1] 9840
# IF-MIB::ifOutUcastPkts[6] 8447505
# IF-MIB::ifOutNUcastPkts[1] 0		seems to always be 0
# IF-MIB::ifOutDiscards[1] 0
# IF-MIB::ifOutErrors[1] 0
# IF-MIB::ifOutQLen[1] 0			seems to always be 0
# IF-MIB::ifSpecific[1] SNMPv2-SMI::zeroDotZero
def process_interfaces(admin_ip, data, contents, context):
	#dump_snmp(admin_ip, 'interfaces', contents)
	
	# create an interfaces table (we can't build details until we know ip addresses)
	descs = get_values(contents, "ifDescr")
	macs = get_values(contents, "ifPhysAddress")
	speeds = get_values(contents, "ifSpeed")
	mtus = get_values(contents, "ifMtu")
	in_octets = get_values(contents, "ifInOctets")
	out_octets = get_values(contents, "ifOutOctets")
	qlens = get_values(contents, "ifOutQLen")
	status = get_values(contents, "ifOperStatus")
	for index in descs.keys():
		if status.get(index, '') == 'up' or status.get(index, '') == 'dormant':
			entry = {'if_index': index, 'name': descs.get(index, ''), 'mac': sanitize_mac(macs.get(index, '')), 'speed': speeds.get(index, ''), 'mtu': mtus.get(index, ''), 'in_octets': in_octets.get(index, ''), 'out_octets': out_octets.get(index, ''), 'qlen': qlens.get(index, '')}
			if admin_ip in context['interfaces']:
				context['interfaces'][admin_ip].append(entry)
			else:
				context['interfaces'][admin_ip] = [entry]

def add_label(data, target, label, key, level = 0, style = ''):
	if label:
		sort_key = '%s-%s' % (level, key)
		if style:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key, 'style': style})
		else:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key})
		
def add_details(data, target, label, detail, opened, sort_key, key):
	data['details'].append({'entity-id': target, 'label': label, 'detail': detail, 'open': opened, 'sort-key': sort_key, 'id': key})

def add_relation(data, left, right, style = '', left_label = None, middle_label = None, right_label = None):
	relation = {'left-entity-id': left, 'right-entity-id': right, 'style': style}
	if left_label:
		relation['left-label'] = left_label
	if middle_label:
		relation['middle-label'] = middle_label
	if right_label:
		relation['right-label'] = right_label
	data['relations'].append(relation)

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
	expr = re.compile(r'::%s (?= \W) .*? \  (.+)$' % name, re.MULTILINE | re.VERBOSE)	# TODO: faster to cache these
	match = re.search(expr, contents)
	if match:
		return fmt % match.group(1)
	return None

# Matches "MIB::<name>[<key>] <value>" and returns a dict
# mapping keys to values.
def get_values(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[ ([^\]]+) \] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		values[match.group(1)] = match.group(2)
	
	return values

# Matches second bracketed expression:
# IP-MIB::ipNetToMediaPhysAddress[5][10.104.0.254] 0:19:bb:5f:59:8a
def get_values2(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[ [^\]]+ \] \[ ([^\]]+) \] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		values[match.group(1)] = match.group(2)
	
	return values

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
		
# MAC addresses are normally lower case which is kind of ugly and
# also are not always two digits which is also ugly, but even more
# important causes problems when we try to match up information
# from different MIBs.
def sanitize_mac(mac):
	result = []
	for part in mac.split(':'):		# classier to do this with map but lambda are weak in Python
		part = part.upper()
		if len(part) == 1:
			result.append('0' + part)
		else:
			result.append(part)
	return ':'.join(result)

def add_units(table, name, unit):
	result = table.get(name, '')
	if result:
		result += ' ' + unit
	return result
			
def count_leading_ones(mask):
	count = 0
	
	bit = 1 << 31
	while bit > 0:
		if mask & bit == bit:
			count += 1
			bit >>= 1
		else:
			break
	
	return count

def count_trailing_zeros(mask):
	count = 0
	
	bit = 1
	while bit < (1 << 32):
		if mask & bit == 0:
			count += 1
			bit <<= 1
		else:
			break
	
	return count;

def get_subnet(s):
	parts = s.split('.')
	bytes = map(lambda p: int(p), parts)
	mask = reduce(lambda sum, current: 256*sum + current, bytes, 0)
	leading = count_leading_ones(mask)
	trailing = count_trailing_zeros(mask)
	if leading + trailing == 32:
		return leading
	else:
		return s		# unusual netmask where 0s and 1s are mixed.

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
		self.__handlers = {'system': process_system, 'ip': process_ip, 'interfaces': process_interfaces}
		self.__context = {}
	
	def run(self):
		rate = self.__config['poll-rate']
		while time.time() - self.__startTime < self.__args.duration:
			currentTime = time.time()
			if not self.__args.put:
				logger.info("-" * 60)
				
			self.__context['ips'] = {}			# device ip => admin ip
			self.__context['netmasks'] = {}	# device ip => network mask
			self.__context['if_indexes'] = {}	# admin ip + if index => device ip
			self.__context['interfaces'] = {}	# admin ip => [{'if_index':, 'name':, 'mac':, 'speed':, 'mtu':, 'in_octets':, 'out_octets', 'qlen'}]
			self.__context['routes'] = []		# list of {'src':, 'next hop':, 'dest':, 'metric':, 'protocol':}
			
			threads = self.__spawn_threads()
			data = self.__process_threads(threads)
			self.__add_next_hop_relations(data)
			self.__add_interfaces_table(data)
			send_update(self.__config, data)
			
			elapsed = time.time() - currentTime
			logger.info('elapsed: %.1f seconds' % elapsed)
			time.sleep(max(rate - elapsed, 5))
			
	def __add_interfaces_table(self, data):
		for (admin_ip, interfaces) in self.__context['interfaces'].items():
			detail = {}
			detail['style'] = 'html'
			detail['header'] = ['Name', 'IP Address', 'Mac Address', 'Speed', 'MTU', 'In Octets', 'Out Octets', 'Out QLen']
			
			rows = []
			for interface in interfaces:
				name = cgi.escape(interface['name'])
				ip = self.__context['if_indexes'].get(admin_ip + interface['if_index'], '')
				subnet = get_subnet(self.__context['netmasks'].get(ip))
				if ip == admin_ip:
					ip = '<strong>%s/%s</strong>' % (ip, subnet)
				else:
					ip = '%s/%s' % (ip, subnet)
				speed = interface.get('speed', '')
				if speed:
					speed = float(speed)/1000000
					speed = '%.1f Mbps' % speed
				rows.append([name, ip, interface['mac'], speed, add_units(interface, 'mtu', 'B'), add_units(interface, 'in_octets', 'B'), add_units(interface, 'out_octets', 'B'), add_units(interface, 'qlen', 'p')])
			detail['rows'] = sorted(rows, key = lambda row: row[0])
			
			target = 'entities:%s' % admin_ip
			add_details(data, target, 'Interfaces', json.dumps(detail), opened = 'yes', sort_key = 'alpha', key = 'interfaces table')
			
	def __add_next_hop_relations(self, data):
		next_hops = []
		metrics = {}
		protocols = {}
		for route in self.__context['routes']:
			src_ip = route['src']
			if route['next hop'] in self.__context['ips']:
				next_hop = self.__context['ips'][route['next hop']]
				next_hops.append((src_ip, next_hop))
				metrics[(src_ip, next_hop)] = route['metric']
				protocols[(src_ip, next_hop)] = route['protocol']
			
		for (src_ip, next_hop) in next_hops:
			style = None
			right_label = None
			if (next_hop, src_ip) in next_hops:
				if src_ip < next_hop:
					style = 'line-type:bidirectional'
					right_label = '%s, cost %s' % (protocols[(next_hop, src_ip)], metrics[(src_ip, next_hop)])
			else:
				style = 'line-type:directed'
			if style:
				left = 'entities:%s' % src_ip
				right = 'entities:%s' % next_hop
				left_label = {'label': '%s, cost %s' % (protocols[(src_ip, next_hop)], metrics[(src_ip, next_hop)]), 'level': 2}
				if right_label:
					right_label = {'label': right_label, 'level': 2}
				add_relation(data, left, right, style, left_label = left_label, middle_label = {'label': 'next hop', 'level': 1}, right_label = right_label)
	
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
					self.__handlers[mib](thread.ip, data, contents, self.__context)
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
