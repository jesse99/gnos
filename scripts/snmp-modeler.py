#!/usr/bin/python
# This script uses snmp to periodically model a network, encodes it into a json 
# dictionary, and ships the dictionary off to gnos using an http POST.  This
# is designed to be a generic modeler suitable for pretty much any device 
# running SNMP.
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
import cgi, json, itertools, httplib, re, sys, threading, time
from base_modeler import *

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)
	
logger = None
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
def process_system(admin_ip, data, contents, context):
	#dump_snmp(admin_ip, 'system', contents)
	target = 'entities:%s' % admin_ip
	key = 'alpha'		# want these to appear before most other labels
	up_time = get_value(contents, "%s", 'sysUpTime')
	if not up_time:
		up_time = get_value(contents, "%s", 'sysUpTimeInstance')
	if up_time:
		up_time = float(up_time)/100.0
		context['up_times'][admin_ip] = up_time
		add_label(data, target, 'uptime: %s' % secs_to_str(up_time), key, level = 2, style = 'font-size:small')
		if up_time < 60.0:
			# TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
			open_alert(data, target, key = 'uptime', mesg = 'Device rebooted.', resolution = '', kind = 'error')
		else:
			close_alert(data, target, key = 'uptime')
	
	context['system'][admin_ip] += get_value(contents, '* %s\n', 'sysDescr')
	context['system'][admin_ip] += get_value(contents, '* %s\n', 'sysContact')
	context['system'][admin_ip] += get_value(contents, '* location is %s\n', 'sysLocation')
	
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
	target = 'entities:%s' % admin_ip
	key = 'zeppo'
	
	if get_value(contents, '%s', 'ipForwarding') == 'forwarding':
		context['system'][admin_ip] += '* ip forwarding is on\n'
	else:
		context['system'][admin_ip] += '* ip forwarding is off\n'
		
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
# IF-MIB::ifOutQLen[1] 0	
# IF-MIB::ifSpecific[1] SNMPv2-SMI::zeroDotZero
def process_interfaces(admin_ip, data, contents, context):
	target = 'entities:%s' % admin_ip
	
	# create an interfaces table (we can't build details until we know ip addresses)
	descs = get_values(contents, "ifDescr")
	macs = get_values(contents, "ifPhysAddress")
	speeds = get_values(contents, "ifSpeed")
	mtus = get_values(contents, "ifMtu")
	in_octets = get_values(contents, "ifInOctets")
	out_octets = get_values(contents, "ifOutOctets")
	status = get_values(contents, "ifOperStatus")
	found = set()
	for index in descs.keys():
		# This is all kinds of screwed up but when devices are brought up and down multiple
		# entries land in the table. So what we'll do is add the ones that are enabled and
		# then add any that we missed that are down (we need the downed interfaces so
		# that we can store zero samples to keep them lined up).
		if status.get(index, '') == 'up' or status.get(index, '') == 'dormant':
			name = descs.get(index, '')
			entry = {
				'enabled': True,
				'if_index': index,
				'name': name,
				'mac': sanitize_mac(macs.get(index, '')),
				'speed': float(speeds.get(index, 0.0)),
				'mtu': mtus.get(index, ''),
				'in_octets': float(in_octets.get(index, 0.0)),
				'out_octets': float(out_octets.get(index, 0.0))
			}
			found.add(name)
			if admin_ip in context['interfaces']:
				context['interfaces'][admin_ip].append(entry)
			else:
				context['interfaces'][admin_ip] = [entry]
	for index in descs.keys():
		name = descs.get(index, '')
		if status.get(index, '') != 'up' and status.get(index, '') != 'dormant' and name not in found:
			entry = {
				'enabled': False,
				'if_index': index,
				'name': name,
				'mac': sanitize_mac(macs.get(index, '')),
				'speed': float(speeds.get(index, 0.0)),
				'mtu': mtus.get(index, ''),
				'in_octets': 0.0,			# these will often be nonsense
				'out_octets': 0.0
			}
			found.add(name)
			if admin_ip in context['interfaces']:
				context['interfaces'][admin_ip].append(entry)
			else:
				context['interfaces'][admin_ip] = [entry]
				
	# alert if interface operational status doesn't match admin status
	admin_status = get_values(contents, "ifAdminStatus")
	oper_status = get_values(contents, "ifOperStatus")
	for (index, admin) in admin_status.items():
		name = descs.get(index, '?')
		key = '%s-oper-status' % name
		if index in oper_status and admin != oper_status[index] and oper_status[index] != 'dormant':
			mesg = 'Admin set %s to %s but it is %s.' % (name, admin, oper_status[index])
			open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'error')	# TODO: what about resolution?
		else:
			close_alert(data, target, key)
			
	# alert if interface operational status changed recently
	last_changes = get_values(contents, "ifLastChange")
	for (index, last_change) in last_changes.items():
		up_time = float(last_change)/100.0
		if up_time:
			name = descs.get(index, '?')
			context['interface_up_times'][(admin_ip, name)] = (up_time, oper_status.get(index, '?'))

# HOST-RESOURCES-MIB::hrMemorySize.0 246004
# HOST-RESOURCES-MIB::hrStorageIndex[1] 1													one of these for each storage type
# HOST-RESOURCES-MIB::hrStorageType[1] HOST-RESOURCES-TYPES::hrStorageRam 	or hrStorageVirtualMemory, hrStorageOther, hrStorageFixedDisk
# HOST-RESOURCES-MIB::hrStorageDescr[1] Physical memory 									or Virtual memory, Memory buffers, Cached memory, Swap space, /rom, /overlay
# HOST-RESOURCES-MIB::hrStorageAllocationUnits[1] 1024
# HOST-RESOURCES-MIB::hrStorageSize[1] 246004
# HOST-RESOURCES-MIB::hrStorageUsed[1] 177396
def process_storage(admin_ip, data, contents, context):
	target = 'entities:%s' % admin_ip
	storage = get_values(contents, "hrStorageDescr")
	used = get_values(contents, "hrStorageUsed")
	size = get_values(contents, "hrStorageSize")
	units = get_values(contents, "hrStorageAllocationUnits")
	for (index, kind) in storage.items():
		# update system details with info about storage
		unit = float(units.get(index, 0))
		actual = unit*float(size.get(index, 0))/(1024*1024)
		if actual:
			use = unit*float(used.get(index, 0))/(1024*1024)/actual
			context['system'][admin_ip] += '* %s has %.1f MiB with %.0f%% in use\n' % (kind.lower(), actual, 100*use)
		
		# add a gauge if virtual memory is full
		level = None
		if kind == 'Virtual memory':
			value = float(used.get(index, '0'))/float(size.get(index, '1'))
			if value >= 0.95:		# not sure what's bad but I think Linux machines often run with high VM usage
				level = 1
				style = 'gauge-bar-color:salmon'
			elif value >= 0.90:
				level = 2
				style = 'gauge-bar-color:darkorange'
			elif value >= 0.80:
				level = 3
				style = 'gauge-bar-color:skyblue'
			if level:
				add_gauge(data, target, 'virtual memory', value, level, style, sort_key = 'zz')
				
		# add a gauge if the main disk is full
		elif kind == '/':
			value = float(used.get(index, '0'))/float(size.get(index, '1'))
			if value >= 0.90:
				level = 1
				style = 'gauge-bar-color:salmon'
			elif value >= 0.75:
				level = 2
				style = 'gauge-bar-color:darkorange'
			if level:
				add_gauge(data, target, 'disk usage', value, level, style, sort_key = 'zzz')

# HOST-RESOURCES-MIB::hrDeviceIndex[768] 768																one of these for each processor, each network interface, disk, etc
# HOST-RESOURCES-MIB::hrDeviceType[768] HOST-RESOURCES-TYPES::hrDeviceProcessor				or hrDeviceNetwork, hrDeviceDiskStorage
# HOST-RESOURCES-MIB::hrDeviceDescr[768] GenuineIntel: Intel(R) Atom(TM) CPU  330   @ 1.60GHz		or eth2, SCSI disk, etc
# HOST-RESOURCES-MIB::hrDeviceID[768] SNMPv2-SMI::zeroDotZero
# HOST-RESOURCES-MIB::hrDeviceStatus[768] running															or down
# HOST-RESOURCES-MIB::hrDeviceErrors[1025] 0
# HOST-RESOURCES-MIB::hrProcessorFrwID[768] SNMPv2-SMI::zeroDotZero
# HOST-RESOURCES-MIB::hrProcessorLoad[768] 1																only applies to hrDeviceProcessor
# HOST-RESOURCES-MIB::hrNetworkIfIndex[1025] 1															(lot of these are specific to the device type)
# HOST-RESOURCES-MIB::hrDiskStorageAccess[1552] readWrite
# HOST-RESOURCES-MIB::hrDiskStorageMedia[1552] unknown
# HOST-RESOURCES-MIB::hrDiskStorageRemoveble[1552] false
# HOST-RESOURCES-MIB::hrDiskStorageCapacity[1552] 256000
# HOST-RESOURCES-MIB::hrPartitionIndex[1552][1] 1
# HOST-RESOURCES-MIB::hrPartitionLabel[1552][1] "/dev/sda1"
# HOST-RESOURCES-MIB::hrPartitionID[1552][1] "0x801"
# HOST-RESOURCES-MIB::hrPartitionSize[1552][1] 0
# HOST-RESOURCES-MIB::hrPartitionFSIndex[1552][1] 0
# HOST-RESOURCES-MIB::hrFSIndex[1] 1
# HOST-RESOURCES-MIB::hrFSMountPoint[1] "/rom"
# HOST-RESOURCES-MIB::hrFSRemoteMountPoint[1] ""
# HOST-RESOURCES-MIB::hrFSType[1] HOST-RESOURCES-TYPES::hrFSOther
# HOST-RESOURCES-MIB::hrFSAccess[1] readOnly
# HOST-RESOURCES-MIB::hrFSBootable[1] false
# HOST-RESOURCES-MIB::hrFSStorageIndex[1] 31
# HOST-RESOURCES-MIB::hrFSLastFullBackupDate[1] 0-1-1,0:0:0.0
# HOST-RESOURCES-MIB::hrFSLastPartialBackupDate[1] 0-1-1,0:0:0.0
def process_device(admin_ip, data, contents, context):
	descrs = get_values(contents, "hrDeviceDescr")
	status = get_values(contents, "hrDeviceStatus")
	errors = get_values(contents, "hrDeviceErrors")
	for (index, desc) in descrs.items():
		# update system details with info about devices
		stat = status.get(index, '')
		errs = errors.get(index, '0')
		if stat:
			context['system'][admin_ip] += '* %s is %s with %s errors\n' % (desc, stat, errs)
		
	# add a gauge if processor load is high
	load = get_value(contents, '%s', 'hrProcessorLoad')
	if load:
		target = 'entities:%s' % admin_ip
		level = None
		value = int(load)/100.0
		if value >= 0.90:
			level = 1
			style = 'gauge-bar-color:salmon'
		elif value >= 0.75:
			level = 2
			style = 'gauge-bar-color:darkorange'
		elif value >= 0.50:
			level = 3
			style = 'gauge-bar-color:skyblue'
		if level:
			add_gauge(data, target, 'processor load', value, level, style, sort_key = 'y')

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

def add_units(value, unit):
	if value or type(value) == float:
		value = '%s %s' % (value, unit)
	return value

class DeviceThread(threading.Thread):
	def __init__(self, ip, authentication, mib_names):
		threading.Thread.__init__(self)
		self.ip = ip
		self.__authentication = authentication
		self.__mib_names = mib_names
		self.results = None									# mapping from mib name to results of the query for that mib
		
	def run(self):
		self.results = {}
		for name in self.__mib_names:
			self.results[name] = self.__walk_mib(name)
		
	# When only a few items are used it would be faster to use something like:
	# snmpbulkget -v2c -c public 10.101.0.2 -Oq -Ot -OU -OX ipRouteMask ipFragFails ipDefaultTTL
	def __walk_mib(self, name):
		command = 'snmpbulkwalk %s %s -Oq -Ot -OU -OX %s' % (self.__authentication, self.ip, name)
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
		self.__last_time = None
		# TODO: 
		# alert if hrSystemDate is too far from admin machine's datetime
		# might be nice to do something with tcp and udp stats
		self.__handlers = {'system': process_system, 'ip': process_ip, 'interfaces': process_interfaces, 'hrStorage': process_storage, 'hrDevice': process_device}
		self.__context = {}
		self.__num_samples = 0
	
	def run(self):
		rate = self.__config['poll-rate']
		while True:
			self.__current_time = time.time()
			if not self.__args.put:
				logger.info("-" * 60)
				
			self.__context['ips'] = {}			# device ip => admin ip
			self.__context['netmasks'] = {}	# device ip => network mask
			self.__context['if_indexes'] = {}	# admin ip + if index => device ip
			self.__context['interfaces'] = {}	# admin ip => [{'if_index':, 'name':, 'mac':, 'speed':, 'mtu':, 'in_octets':, 'out_octets'}]
			self.__context['routes'] = []		# list of {'src':, 'next hop':, 'dest':, 'metric':, 'protocol':}
			self.__context['system'] = {}		# admin ip => markdown with system info details
			self.__context['up_times'] = {}	# admin_ip => system up time
			self.__context['interface_up_times'] = {}	# (admin_ip, ifname) => (interface up time, interface status)
			
			threads = self.__spawn_threads()
			data = self.__process_threads(threads)
			self.__add_next_hop_relations(data)
			self.__add_interfaces_table(data)
			if self.__num_samples >= 2:
				self.__add_bandwidth_chart(data, 'out')
				self.__add_bandwidth_chart(data, 'in')
				self.__add_bandwidth_details(data, 'out')
				self.__add_bandwidth_details(data, 'in')
			self.__add_system_info(data);
			self.__add_interface_uptime_alert(data)
			send_update(self.__config, connection, data)
			
			elapsed = time.time() - self.__current_time
			self.__last_time = self.__current_time
			self.__num_samples += 1
			logger.info('elapsed: %.1f seconds' % elapsed)
			if time.time() - self.__startTime < self.__args.duration:
				time.sleep(max(rate - elapsed, 5))
			else:
				break
			
	def __add_interfaces_table(self, data):
		for (admin_ip, interfaces) in self.__context['interfaces'].items():
			detail = {}
			detail['style'] = 'html'
			detail['header'] = ['Name', 'IP Address', 'Mac Address', 'Speed', 'MTU', 'In Octets (kbps)', 'Out Octets (kbps)']
			
			rows = []
			for interface in interfaces:
				name = cgi.escape(interface['name'])
				
				ip = self.__context['if_indexes'].get(admin_ip + interface['if_index'], '')
				subnet = get_subnet(self.__context['netmasks'].get(ip))
				if ip == admin_ip:
					ip = '<strong>%s/%s</strong>' % (ip, subnet)
				else:
					ip = '%s/%s' % (ip, subnet)
				
				# We always need to add samples so that they stay in sync with one another.
				in_octets = self.__process_sample(data, {'key': '%s-%s-in_octets' % (admin_ip, name), 'raw': 8*interface['in_octets']/1000, 'units': 'kbps'})
				out_octets = self.__process_sample(data, {'key':  '%s-%s-out_octets' % (admin_ip, name), 'raw': 8*interface['out_octets']/1000, 'units': 'kbps'})
				
				if interface['enabled']:
					speed = interface.get('speed', 0.0)
					if speed:
						if out_octets['value']:
							self.__add_interface_gauge(data, admin_ip, name, out_octets['value'], speed/1000)
						speed = speed/1000000
						speed = '%.1f Mbps' % speed
						
					rows.append([name, ip, interface['mac'], speed, add_units(interface['mtu'], 'B'), in_octets['html'], out_octets['html']])
			detail['rows'] = sorted(rows, key = lambda row: row[0])
			
			target = 'entities:%s' % admin_ip
			footnote = '*The shaded area in the sparklines is the inter-quartile range: the range in which half the samples appear.*'
			add_details(data, target, 'Interfaces', [detail, footnote], opened = 'yes', sort_key = 'alpha', key = 'interfaces table')
			
	def __add_bandwidth_chart(self, data, direction):
		for (admin_ip, interfaces) in self.__context['interfaces'].items():
			samples = []
			legends = []
			table = sorted(interfaces, key = lambda i: i['name'])
			for interface in table:
				if interface['enabled']:
					name = interface['name']
					legends.append(name)
					samples.append('%s-%s-%s_octets' % (admin_ip, name, direction))
			
			name = "%s-%s_interfaces" % (admin_ip, direction)
			data['charts'].append({'name': name, 'samples': samples, 'legends': legends, 'title': '%s Bandwidth' % direction.title(), 'y_label': 'Bandwidth (kbps)'})
		
	def __add_bandwidth_details(self, data, direction):
		for admin_ip in self.__context['interfaces'].keys():
			target = 'entities:%s' % admin_ip
			name = "%s-%s_interfaces" % (admin_ip, direction)
			markdown = '![bandwidth](/generated/%s.png#%s)' % (name, self.__num_samples)
			add_details(data, target, '%s Bandwidth' % direction.title(), [markdown], opened = 'no', sort_key = 'alpha-' + direction, key = '%s bandwidth' % name)
			
	def __add_system_info(self, data):
		for (admin_ip, markdown) in self.__context['system'].items():
			target = 'entities:%s' % admin_ip
			add_details(data, target, 'System Info', [markdown], opened = 'no', sort_key = 'beta', key = 'system info')
			
	def __add_interface_uptime_alert(self, data):
		for (key, value) in self.__context['interface_up_times'].items():
			(admin_ip, ifname) = key
			(interface_uptime, status) = value
			
			delta = self.__context['up_times'].get(admin_ip, 0.0) - interface_uptime
			key = '%s-last-change' % ifname
			target = 'entities:%s' % admin_ip
			if delta >= 0.0 and delta < 60.0:
				mesg = '%s status recently changed to %s.' % (ifname, status)
				open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'warning')
			else:
				close_alert(data, target, key)
		
	def __add_interface_gauge(self, data, admin_ip, ifname, out_octets, speed):
		level = None
		bandwidth = min(out_octets/speed, 1.0)
		if bandwidth >= 0.75:
			level = 1
			style = 'gauge-bar-color:salmon'
		elif bandwidth >= 0.50:
			level = 1
			style = 'gauge-bar-color:darkorange'
		elif bandwidth >= 0.25:
			level = 2
			style = 'gauge-bar-color:skyblue'
		elif bandwidth >= 0.10:
			level = 4
			style = 'gauge-bar-color:limegreen'
		if level:
			target = 'entities:%s' % admin_ip
			add_gauge(data, target, '%s bandwidth' % ifname, bandwidth, level, style, sort_key = 'z')
		
	# Bit of an ugly function: it does four different things:
	# 1) It computes the current sample value. If a per sec value cannot be computed zero is used
	# (we need to always record a sample value so that the various sample sets align).
	# 2) It ships the new sample off to the server.
	# 3) An html entry is initialized with either a blank value or an url to a sparkline chart for the sample.
	# 4) The sample value and html link are returned to out caller.
	def __process_sample(self, data, table):
		# On input table has: key, raw, and units
		# On exit: value and html are added
		table['value'] = None
		table['html'] = ''
		value = 0.0
		if self.__last_time and self.__context.get(table['key'], 0.0) > 0.0:
			elapsed = self.__current_time - self.__last_time
			if elapsed > 1.0:
				value = (table['raw'] - self.__context[table['key']])/elapsed
		table['value'] = value
		if self.__num_samples >= 2:
			data['samples'].append({'name': table['key'], 'value': value, 'units': table['units']})
		
		# When dynamically adding html content browsers will not reload images that have
		# been already loaded. To work around this we add a unique fragment identifier
		# which the server will ignore.
		if self.__num_samples >= 2:
			url = '/generated/%s.png#%s' % (table['key'], self.__num_samples)
			table['html'] = "<img src = '%s' alt = '%s'>" % (url, table['key'])
		
		self.__context[table['key']] = table['raw']
		return table
		
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
				left_label = {'label': '%s, cost %s' % (protocols[(src_ip, next_hop)], metrics[(src_ip, next_hop)]), 'level': 2, 'style': 'font-size:x-small'}
				if right_label:
					right_label = {'label': right_label, 'level': 2, 'style': 'font-size:x-small'}
				predicate = 'options.next_hop'
				add_relation(data, left, right, style, left_label = left_label, middle_label = {'label': 'next hop', 'level': 1, 'style': 'font-size:small'}, right_label = right_label, predicate = predicate)
	
	# Devices can have significant variation in how quickly they respond to SNMP queries
	# so simply joining them one after another isn't great, but it's simple and should work
	# fine most of the time.
	def __process_threads(self, threads):
		data = {'modeler': 'snmp', 'entities': [], 'relations': [], 'labels': [], 'gauges': [], 'details': [], 'alerts': [], 'samples': [], 'charts': []}
		for thread in threads:
			thread.join(3.0)
			
			target = 'entities:%s' % thread.ip
			if not thread.isAlive():
				close_alert(data, target, key = 'device down')
				for (mib, contents) in thread.results.items():
					if thread.ip not in self.__context['system']:
						self.__context['system'][thread.ip] = ''
					self.__handlers[mib](thread.ip, data, contents, self.__context)
			else:
				open_alert(data, target, key = 'device down', mesg = 'Device is down.', resolution = 'Check the power cable, power it on if it is off, check the IP address, verify routing.', kind = 'error')
		return data
	
	# This could be a lot of threads but they spend nearly all their time blocked so
	# that should be OK.
	def __spawn_threads(self):
		threads = []
		for (name, device) in self.__config["devices"].items():
			if device['modeler'] == 'snmp-modeler.py':
				thread = DeviceThread(device['ip'], device['authentication'], self.__handlers.keys())
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
logger = configure_logging(args, 'snmp-modeler.log')

# Read config info.
config = None
with open(args.config, 'r') as f:
	config = json.load(f)
	
if args.put:
	address = "%s:%s" % (config['server'], config['port'])
	connection = httplib.HTTPConnection(address, strict = True, timeout = 10)

try:
	# Send entity information to the server. TODO: only one modeler should do this,
	# maybe use a command-line option?
	send_entities(config, connection)
	
	# Start polling each device.
	poller = Poll(args, config)
	poller.run()
finally:
	if connection:
		connection.close()
