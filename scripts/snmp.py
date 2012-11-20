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
#
# Following site has a bunch of snmp links: http://www.wtcs.org/snmp4tpc/literature.htm
import cgi, json, itertools, httplib, re, sys, threading, time
from helpers import *
from net_types import *

connection = None

def find_interface(device, ifindex):
	for candidate in device.interfaces:
		if candidate.index == ifindex:
			return candidate
	
	interface = Interface()
	interface.admin_ip = device.admin_ip
	interface.index = ifindex
	device.interfaces.append(interface)
	return interface
	
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
def process_system(data, contents, query):
	#dump_snmp(query.device.admin_ip, 'system', contents)
	up_time = get_value(contents, "%s", 'sysUpTime')
	if not up_time:
		up_time = get_value(contents, "%s", 'sysUpTimeInstance')
	if up_time:
		query.device.uptime = float(up_time)/100.0
	
	query.device.system_info += get_value(contents, '* %s\n', 'sysDescr')
	query.device.system_info += get_value(contents, '* %s\n', 'sysContact')
	
	loc = get_value(contents, '%s', 'sysLocation')
	if loc:
		query.device.system_info += '* location is %s\n' % loc
	
# IP-MIB::ipAdEntAddr[10.0.4.2] 10.0.4.2			will be one of these for each interface
# IP-MIB::ipAdEntIfIndex[10.0.4.2] 7
# IP-MIB::ipAdEntNetMask[10.0.4.2] 255.255.255.0
# IP-MIB::ipAdEntBcastAddr[10.0.4.2] 1
# IP-MIB::ipNetToMediaIfIndex[5][10.104.0.254] 5
# IP-MIB::ipNetToMediaPhysAddress[5][10.104.0.254] 0:19:bb:5f:59:8a
# IP-MIB::ipNetToMediaNetAddress[5][10.104.0.254] 10.104.0.254
# IP-MIB::ipNetToMediaType[5][10.104.0.254] dynamic
def process_ip_addr(data, contents, query):
	indexes = get_values(contents, "ipAdEntIfIndex")
	masks = get_values(contents, "ipAdEntNetMask")
	for ip in indexes.keys():
		index = indexes.get(ip, '?')
		interface = find_interface(query.device, index)
		interface.ip = ip
		interface.mask = masks.get(ip, '?')
		
# RFC1213-MIB::ipRouteDest[10.0.4.0] 10.0.4.0	one for each route (linux)
# RFC1213-MIB::ipRouteIfIndex[10.0.4.0] 7
# RFC1213-MIB::ipRouteMetric1[10.0.4.0] 0
# RFC1213-MIB::ipRouteNextHop[10.0.4.0] 0.0.0.0
# RFC1213-MIB::ipRouteType[10.0.4.0] direct
# RFC1213-MIB::ipRouteProto[10.0.4.0] local
# RFC1213-MIB::ipRouteMask[10.0.4.0] 255.255.255.0
# RFC1213-MIB::ipRouteInfo[10.0.4.0] SNMPv2-SMI::zeroDotZero
def process_ip_route(data, contents, query):
	nexts = get_values(contents, "ipRouteNextHop")
	metrics = get_values(contents, "ipRouteMetric1")
	protocols = get_values(contents, "ipRouteProto")	
	masks = get_values(contents, "ipRouteMask")
	indexes = get_values(contents, "ipRouteIfIndex")
	for dest_ip in nexts.keys():
		route = Route()
		route.via_ip = nexts.get(dest_ip, '')
		route.dst_subnet = dest_ip
		route.dst_mask = masks.get(dest_ip, '')
		route.protocol = protocols.get(dest_ip, '')
		route.metric = metrics.get(dest_ip, '')
		route.ifindex = indexes.get(dest_ip, '')
		
		query.device.routes.append(route)
		
# IP-FORWARD-MIB::ipCidrRouteDest[17.11.12.0][255.255.255.0][0][0.0.0.0] 17.11.12.0		one for each route (cisco)
# IP-FORWARD-MIB::ipCidrRouteMask[17.11.12.0][255.255.255.0][0][0.0.0.0] 255.255.255.0
# IP-FORWARD-MIB::ipCidrRouteTos[17.11.12.0][255.255.255.0][0][0.0.0.0] 0
# IP-FORWARD-MIB::ipCidrRouteNextHop[17.11.12.0][255.255.255.0][0][0.0.0.0] 0.0.0.0
# IP-FORWARD-MIB::ipCidrRouteIfIndex[17.11.12.0][255.255.255.0][0][0.0.0.0] 24
# IP-FORWARD-MIB::ipCidrRouteType[17.11.12.0][255.255.255.0][0][0.0.0.0] local
# IP-FORWARD-MIB::ipCidrRouteProto[17.11.12.0][255.255.255.0][0][0.0.0.0] local
# IP-FORWARD-MIB::ipCidrRouteAge[17.11.12.0][255.255.255.0][0][0.0.0.0] 366040
# IP-FORWARD-MIB::ipCidrRouteInfo[17.11.12.0][255.255.255.0][0][0.0.0.0] SNMPv2-SMI::zeroDotZero
# IP-FORWARD-MIB::ipCidrRouteNextHopAS[17.11.12.0][255.255.255.0][0][0.0.0.0] 0
# IP-FORWARD-MIB::ipCidrRouteMetric1[17.11.12.0][255.255.255.0][0][0.0.0.0] 0
# IP-FORWARD-MIB::ipCidrRouteMetric2[17.11.12.0][255.255.255.0][0][0.0.0.0] -1
# IP-FORWARD-MIB::ipCidrRouteMetric3[17.11.12.0][255.255.255.0][0][0.0.0.0] -1
# IP-FORWARD-MIB::ipCidrRouteMetric4[17.11.12.0][255.255.255.0][0][0.0.0.0] -1
# IP-FORWARD-MIB::ipCidrRouteMetric5[17.11.12.0][255.255.255.0][0][0.0.0.0] -1
# IP-FORWARD-MIB::ipCidrRouteStatus[17.11.12.0][255.255.255.0][0][0.0.0.0] active
def process_ip_cidr(data, contents, query):
	nexts = get_values3(contents, "ipCidrRouteNextHop")
	metrics = get_values3(contents, "ipCidrRouteMetric1")
	protocols = get_values3(contents, "ipCidrRouteProto")
	masks = get_values3(contents, "ipCidrRouteMask")
	indexes = get_values3(contents, "ipCidrRouteIfIndex")
	status = get_values3(contents, "ipCidrRouteStatus")
	for dest_ip in nexts.keys():
		if status.get(dest_ip, '') == 'active':
			route = Route()
			route.via_ip = nexts.get(dest_ip, '')
			route.dst_subnet = dest_ip
			route.dst_mask = masks.get(dest_ip, '')
			route.protocol = protocols.get(dest_ip, '')
			route.metric = metrics.get(dest_ip, '')
			route.ifindex = indexes.get(dest_ip, '')
			
			query.device.routes.append(route)
		
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
def process_interfaces(data, contents, query):
	# update interfaces
	descs = get_values(contents, "ifDescr")
	macs = get_values(contents, "ifPhysAddress")
	speeds = get_values(contents, "ifSpeed")
	mtus = get_values(contents, "ifMtu")
	in_octets = get_values(contents, "ifInOctets")
	out_octets = get_values(contents, "ifOutOctets")
	status = get_values(contents, "ifOperStatus")
	last_changes = get_values(contents, "ifLastChange")
	found = set()
	for index in descs.keys():
		# This is all kinds of screwed up but when devices are brought up and down multiple
		# entries land in the table. So what we'll do is add the ones that are enabled and
		# then add any that we missed that are down.
		if status.get(index, '') == 'up' or status.get(index, '') == 'dormant':
			name = descs.get(index, '').replace('/', '_')	# Ciscos use ifnames like FastEthernet0/0.8 which can cause problems with file paths and urls
			found.add(name)
			
			interface = find_interface(query.device, index)
			interface.name = name
			interface.status = status.get(index, '')
			interface.mac_addr = sanitize_mac(macs.get(index, ''))
			interface.speed = float(speeds.get(index, 0.0))
			interface.mtu = int(mtus.get(index, ''))
			interface.in_octets = float(in_octets.get(index, 0.0))
			interface.out_octets = float(out_octets.get(index, 0.0))
			interface.last_changed = float(last_changes.get(index, 0.0))/100.0
			
	for index in descs.keys():
		name = descs.get(index, '').replace('/', '_')
		if status.get(index, '') != 'up' and status.get(index, '') != 'dormant' and name not in found:
			found.add(name)
			
			interface = find_interface(query.device, index)
			interface.name = name
			interface.status = status.get(index, '')
			interface.mac_addr = sanitize_mac(macs.get(index, ''))
			interface.speed = float(speeds.get(index, 0.0))
			interface.mtu = int(mtus.get(index, ''))
			interface.in_octets = 0.0 				# these will often be nonsense
			interface.out_octets = 0.0
			interface.last_changed = float(last_changes.get(index, 0.0))/100.0
	
	# alert if operational status doesn't match admin status
	target = 'entities:%s' % query.device.admin_ip
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

# HOST-RESOURCES-MIB::hrMemorySize.0 246004
# HOST-RESOURCES-MIB::hrStorageIndex[1] 1													one of these for each storage type
# HOST-RESOURCES-MIB::hrStorageType[1] HOST-RESOURCES-TYPES::hrStorageRam 	or hrStorageVirtualMemory, hrStorageOther, hrStorageFixedDisk
# HOST-RESOURCES-MIB::hrStorageDescr[1] Physical memory 									or Virtual memory, Memory buffers, Cached memory, Swap space, /rom, /overlay
# HOST-RESOURCES-MIB::hrStorageAllocationUnits[1] 1024
# HOST-RESOURCES-MIB::hrStorageSize[1] 246004
# HOST-RESOURCES-MIB::hrStorageUsed[1] 177396
def process_storage(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
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
			query.device.system_info += '* %s has %.1f MiB with %.0f%% in use\n' % (kind.lower(), actual, 100*use)
		
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

# CISCO-FLASH-MIB::ciscoFlashPartitionName[1][1] flash:1    one for each flash partition
# CISCO-FLASH-MIB::ciscoFlashPartitionSize[1][1] 64016384
# CISCO-FLASH-MIB::ciscoFlashPartitionFreeSpace[1][1] 802816
# CISCO-FLASH-MIB::ciscoFlashPartitionFileCount[1][1] 29
# CISCO-FLASH-MIB::ciscoFlashPartitionChecksumAlgorithm[1][1] undefined
# CISCO-FLASH-MIB::ciscoFlashPartitionStatus[1][1] readWrite
# CISCO-FLASH-MIB::ciscoFlashPartitionUpgradeMethod[1][1] direct
# CISCO-FLASH-MIB::ciscoFlashPartitionNeedErasure[1][1] false
# CISCO-FLASH-MIB::ciscoFlashPartitionFileNameLength[1][1] 63
# CISCO-FLASH-MIB::ciscoFlashPartitionStartChip[1][1] 1
# CISCO-FLASH-MIB::ciscoFlashPartitionEndChip[1][1] 1
# 
# CISCO-FLASH-MIB::ciscoFlashFileSize[1][1][1] 2026     one for each flash file
# CISCO-FLASH-MIB::ciscoFlashFileChecksum[1][1][1] "0x0"
# CISCO-FLASH-MIB::ciscoFlashFileStatus[1][1][1] valid
# CISCO-FLASH-MIB::ciscoFlashFileName[1][1][1] run-preupgrade-r2
# CISCO-FLASH-MIB::ciscoFlashFileType[1][1][1] unknown
def process_flash(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	names = get_values2(contents, "ciscoFlashPartitionName")
	size = get_values2(contents, "ciscoFlashPartitionSize")
	frees = get_values2(contents, "ciscoFlashPartitionFreeSpace")
	for (key, name) in names.items():
		actual = float(size.get(key, 0))/(1024*1024)
		free = float(frees.get(key, 0))/(1024*1024)
		if actual and free:
			# update system details with info about storage
			used = actual - free
			use = used/actual
			query.device.system_info += '* %s has %.1f MiB with %.0f%% in use\n' % (name, actual, 100*use)
			
			# add a gauge if a partition is full
			level = None
			if use >= 0.90:
				level = 1
				style = 'gauge-bar-color:salmon'
			elif use >= 0.80:
				level = 2
				style = 'gauge-bar-color:darkorange'
			elif use >= 0.75:
				level = 3
				style = 'gauge-bar-color:skyblue'
			if level:
				add_gauge(data, target, name, use, level, style, sort_key = 'zz')

#  CISCO-ENVMON-MIB::ciscoEnvMonTemperatureStatusDescr[1] chassis
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureStatusValue[1] 23
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureThreshold[1] 65
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureState[1] normal		or warning, critical, shutdown, notPresent, notFunctioning
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureLastShutdown[1] 0
def process_temp(data, contents, query):
	#dump_snmp(query.device.admin_ip, 'temp', contents)
	target = 'entities:%s' % query.device.admin_ip
	names = get_values(contents, "ciscoEnvMonTemperatureStatusDescr")
	status = get_values(contents, "ciscoEnvMonTemperatureStatusValue")
	threshold = get_values(contents, "ciscoEnvMonTemperatureThreshold")
	states = get_values(contents, "ciscoEnvMonTemperatureState")
	for (key, name) in names.items():
		current = int(status.get(key, 0))*9/5 + 32
		maximum = int(threshold.get(key, 0))*9/5 + 32
		if current and maximum:
			# update system details with info about temperature
			state = states.get(key, '')
			query.device.system_info += '* %s is %s F (%s). Shutdown will happen at %s F.\n' % (name, current, state, maximum)
			
			# add an alert if the temperature is too high
			key = '%s-temp' % name
			if state == 'critical' or state == 'shutdown':
				mesg = '%s temperature is %s F, shutdown will happen at %s F (%s).' % (name, current, maximum, state)
				open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'error')
			elif state == 'warning':
				mesg = '%s temperature is %s F, shutdown will happen at %s F.' % (name, current, maximum)
				open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'warning')
			else:
				close_alert(data, target, key)

# CISCO-ENVMON-MIB::ciscoEnvMonFanStatusDescr[1] Fan 1		one for each fan
# CISCO-ENVMON-MIB::ciscoEnvMonFanState[1] normal			or warning, critical, shutdown, notPresent, notFunctioning
def process_fan(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	names = get_values(contents, "ciscoEnvMonFanStatusDescr")
	states = get_values(contents, "ciscoEnvMonFanState")
	for (key, name) in names.items():
		state = states.get(key, '')
		if state:
			key = '%s-fan' % name
			if state == 'critical' or state == 'shutdown':
				mesg = '%s state is %s.' % (name, state)
				open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'error')
			elif state == 'warning':
				mesg = '%s state is %s.' % (name, state)
				open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'warning')
			else:
				close_alert(data, target, key)

# CISCO-PROCESS-MIB::cpmCPUTotalPhysicalIndex[1] 0
# CISCO-PROCESS-MIB::cpmCPUTotal5sec[1] 2
# CISCO-PROCESS-MIB::cpmCPUTotal1min[1] 2
# CISCO-PROCESS-MIB::cpmCPUTotal5min[1] 2
# CISCO-PROCESS-MIB::cpmCPUTotal5secRev[1] 2
# CISCO-PROCESS-MIB::cpmCPUTotal1minRev[1] 2
# CISCO-PROCESS-MIB::cpmCPUTotal5minRev[1] 2
# CISCO-PROCESS-MIB::cpmCPUMonInterval[1] 5
# CISCO-PROCESS-MIB::cpmCPUTotalMonIntervalValue[1] 2
# CISCO-PROCESS-MIB::cpmCPUInterruptMonIntervalValue[1] 0
def process_cpu(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	cpu = get_values(contents, "cpmCPUTotal1minRev")
	for (key, v) in cpu.items():
		value = float(v)/100.0
		level = None
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

# CISCO-ENTITY-EXT-MIB::ceExtProcessorRam[3] 268435456
# CISCO-ENTITY-EXT-MIB::ceExtNVRAMSize[3] 245752
# CISCO-ENTITY-EXT-MIB::ceExtNVRAMUsed[3] 42847
def process_nvram(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	sizes = get_values(contents, "ceExtNVRAMSize")
	used = get_values(contents, "ceExtNVRAMUsed")
	for (key, s) in sizes.items():
		size = float(s)
		using = float(used.get(key, '0'))
		if size and using:
			value = using/size
			level = None
			if value >= 0.80:
				level = 1
				style = 'gauge-bar-color:salmon'
			elif value >= 0.75:
				level = 2
				style = 'gauge-bar-color:darkorange'
			elif value >= 0.50:
				level = 3
				style = 'gauge-bar-color:skyblue'
			if level:
				add_gauge(data, target, 'nvram', value, level, style, sort_key = 'zz')

# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolType[1][1] processorMemory		one for each pool
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolName[1][1] Processor
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolPlatformMemory[1][1] SNMPv2-SMI::zeroDotZero
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolAlternate[1][1] 0
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolValid[1][1] true
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolUsed[1][1] 40460516
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolFree[1][1] 113402652
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolLargestFree[1][1] 83889816
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolLowestFree[1][1] 83741284
def process_mempool(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	names = get_values(contents, "cempMemPoolType")
	useds = get_values(contents, "cempMemPoolUsed")
	frees = get_values(contents, "cempMemPoolFree")
	for (key, name) in names.items():
		used = float(useds.get(key, '0'))
		free = float(frees.get(key, '0'))
		if used and free:
			value = used/(used + free)
			level = None
			if value >= 0.80:
				level = 1
				style = 'gauge-bar-color:salmon'
			elif value >= 0.75:
				level = 2
				style = 'gauge-bar-color:darkorange'
			elif value >= 0.50:
				level = 3
				style = 'gauge-bar-color:skyblue'
			if level:
				add_gauge(data, target, name, value, level, style, sort_key = 'zz')

# OSPF-MIB::ospfLsdbAreaId[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 0.0.0.0     one for each link type/advert ip/router id
# OSPF-MIB::ospfLsdbType[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] routerLink   or asExternalLink
# OSPF-MIB::ospfLsdbLsid[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 172.20.254.10
# OSPF-MIB::ospfLsdbRouterId[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 172.20.254.10
# OSPF-MIB::ospfLsdbSequence[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] -2147481898
# OSPF-MIB::ospfLsdbAge[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 1398
# OSPF-MIB::ospfLsdbChecksum[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 11713
# OSPF-MIB::ospfLsdbAdvertisement[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] "00 00..."
def process_ospf_lsdb(data, contents, query):
	dump_snmp(query.device.admin_ip, 'ospf', contents)

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
def process_device(data, contents, query):
	descrs = get_values(contents, "hrDeviceDescr")
	status = get_values(contents, "hrDeviceStatus")
	errors = get_values(contents, "hrDeviceErrors")
	for (index, desc) in descrs.items():
		# update system details with info about devices
		stat = status.get(index, '')
		errs = errors.get(index, '0')
		if stat:
			query.device.system_info += '* %s is %s with %s errors\n' % (desc, stat, errs)
		
	# add a gauge if processor load is high
	load = get_value(contents, '%s', 'hrProcessorLoad')
	if load:
		target = 'entities:%s' % query.device.admin_ip
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
		value = match.group(1)
		if value and value[0] == '"':		# TODO: on cisco's stuff like sysDescr can be multi-line which causes some problems here
			value = value[1:]
		if value and value[-1] == '"':
			value = value[:-1]
		return fmt % value
	return None

# Matches "MIB::<name>[<key>] <value>" and returns a dict
# mapping keys to values.
def get_values(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[ ([^\]]+) \] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		value = match.group(2)
		if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
			value = value[1:-1]
		values[match.group(1)] = value
	
	return values

# Returns a dict mapping the two keys to values.
# CISCO-FLASH-MIB::ciscoFlashPartitionFileCount[1][1] 29
def get_values2(contents, name):
	values = {}
	
	expr = re.compile(r'::%s (\[[^\]]+\] \[[^\]]+\]) \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		values[match.group(1)] = match.group(2)
	
	return values

# Returns a dict mapping the first key to values.
# IP-FORWARD-MIB::ipCidrRouteIfIndex[17.11.12.0][255.255.255.0][0][0.0.0.0] 24
def get_values3(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[([^\]]+)\] \[[^\]]+\] \[[^\]]+\] \[[^\]]+\] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		values[match.group(1)] = match.group(2)
	
	return values

def dump_snmp(ip, name, contents):
	env.logger.debug('%s %s:' % (ip, name))
	env.logger.debug('%s' % (contents))
	
# We'd like a consistent look for the MIBs (which is nice for users and required when
# matching up macs across devices).
def sanitize_mac(mac):
	result = []
	if ' ' in mac:
		# cisco's separate the octets with spaces
		for part in mac.strip().split(' '):
			part = part.upper()
			if len(part) == 1:
				result.append('0' + part)
			else:
				result.append(part)
	else:
		# linux uses colons but uses lower cases and single digits
		for part in mac.split(':'):		# classier to do this with map but lambda are weak in Python
			part = part.upper()
			if len(part) == 1:
				result.append('0' + part)
			else:
				result.append(part)
		
	return ':'.join(result)

class QueryDevice(object):
	def __init__(self, device):
		# TODO: 
		# alert if hrSystemDate is too far from admin machine's datetime (5min maybe)
		# might be nice to do something with tcp and udp stats
		self.__mibs = ['system', 'ipAddrTable', 'interfaces']
		for mib in device.config.get('mibs', '').split(' '):
			if mib == 'cisco-router':
				add_if_missing(self.__mibs, 'ciscoFlashPartitions')
				add_if_missing(self.__mibs, 'ciscoEnvMonTemperatureStatusTable')
				add_if_missing(self.__mibs, 'ciscoEnvMonFanStatusTable')
				add_if_missing(self.__mibs, 'cpmCPUTotalTable')
				add_if_missing(self.__mibs, 'ceExtPhysicalProcessorTable')
				add_if_missing(self.__mibs, 'cempMemPoolTable')
				add_if_missing(self.__mibs, 'ipCidrRouteTable')
				add_if_missing(self.__mibs, 'ospfLsdbTable')
			elif mib == 'linux-router' or  mib == 'linux-host':
				add_if_missing(self.__mibs, 'ipRouteTable')
				add_if_missing(self.__mibs, 'hrStorage')
				add_if_missing(self.__mibs, 'hrDevice')
			else:
				if mib in self.__handlers:
					add_if_missing(self.__mibs, mib)
				else:
					env.logger.error("Don't know how to parse %s mib" % mib)
		self.__handlers = {
			'system': process_system,
			'ipAddrTable': process_ip_addr,
			'ipRouteTable': process_ip_route,
			'ipCidrRouteTable': process_ip_cidr,
			'interfaces': process_interfaces,
			'hrStorage': process_storage,
			'hrDevice': process_device,
			'ciscoFlashPartitions': process_flash,
			'ciscoEnvMonTemperatureStatusTable': process_temp,
			'ciscoEnvMonFanStatusTable': process_fan,
			'cpmCPUTotalTable': process_cpu,
			'ceExtPhysicalProcessorTable': process_nvram,
			'cempMemPoolTable': process_mempool,
			'ospfLsdbTable': process_ospf_lsdb,
		}
		self.device = device
	
	def run(self):
		self.__results = {}
		try:
			# When only a few items are used it would be faster to use something like:
			# snmpbulkget -v2c -c public 10.101.0.2 -Oq -Ot -OU -OX ipRouteMask ipFragFails ipDefaultTTL
			for name in self.__mibs:
				command = 'snmpbulkwalk %s %s -Oq -Ot -OU -OX %s' % (self.device.config['authentication'], self.device.admin_ip, name)
				result = run_process(command)
				if result:
					self.__results[name] = result
		except:
			env.logger.error("snmpwalk failed for %s" % self.device.admin_ip, exc_info = True)
			pass
	
	def process(self, data):
		target = 'entities:%s' % self.device.admin_ip
		if self.__results:
			close_alert(data, target, key = 'device down')
			for (mib, contents) in self.__results.items():
				self.__handlers[mib](data, contents, self)
		else:
			open_alert(data, target, key = 'device down', mesg = 'Device is down.', resolution = 'Check the power cable, power it on if it is off, check the IP address, verify routing.', kind = 'error')
