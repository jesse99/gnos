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
# MIB browser: http://tools.cisco.com/Support/SNMP/do/BrowseOID.do?local=en&substep=2&translate=Translate&tree=NO
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
def process_misc(data, contents, query):
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
	
	if get_value(contents, '%s', 'ipForwarding') == 'forwarding':
		query.device.system_info += '* ip forwarding is on\n'
	else:
		query.device.system_info += '* ip forwarding is off\n'
		
	
# key = [ipAdEntAddr]
# IP-MIB::ipAdEntAddr[10.0.4.2] 10.0.4.2
# IP-MIB::ipAdEntIfIndex[10.0.4.2] 7
# IP-MIB::ipAdEntNetMask[10.0.4.2] 255.255.255.0
# IP-MIB::ipAdEntBcastAddr[10.0.4.2] 1
# IP-MIB::ipNetToMediaIfIndex[5][10.104.0.254] 5
# IP-MIB::ipNetToMediaPhysAddress[5][10.104.0.254] 0:19:bb:5f:59:8a
# IP-MIB::ipNetToMediaNetAddress[5][10.104.0.254] 10.104.0.254
# IP-MIB::ipNetToMediaType[5][10.104.0.254] dynamic
def process_ip_addr(data, contents, query):
	indexes = get_values1(contents, "ipAdEntIfIndex")
	masks = get_values1(contents, "ipAdEntNetMask")
	for ip in indexes.keys():
		index = indexes.get(ip, '?')
		interface = find_interface(query.device, index)
		interface.ip = ip
		interface.mask = masks.get(ip, '?')
		
# key = [ipRouteDest]
# RFC1213-MIB::ipRouteDest[10.0.4.0] 10.0.4.0
# RFC1213-MIB::ipRouteIfIndex[10.0.4.0] 7
# RFC1213-MIB::ipRouteMetric1[10.0.4.0] 0
# RFC1213-MIB::ipRouteNextHop[10.0.4.0] 0.0.0.0
# RFC1213-MIB::ipRouteType[10.0.4.0] direct
# RFC1213-MIB::ipRouteProto[10.0.4.0] local
# RFC1213-MIB::ipRouteMask[10.0.4.0] 255.255.255.0
# RFC1213-MIB::ipRouteInfo[10.0.4.0] SNMPv2-SMI::zeroDotZero
def process_ip_route(data, contents, query):
	nexts = get_values1(contents, "ipRouteNextHop")
	metrics = get_values1(contents, "ipRouteMetric1")
	protocols = get_values1(contents, "ipRouteProto")	
	masks = get_values1(contents, "ipRouteMask")
	indexes = get_values1(contents, "ipRouteIfIndex")
	for dest_ip in nexts.keys():
		route = Route()
		route.via_ip = nexts.get(dest_ip, '')
		route.dst_subnet = dest_ip
		route.dst_mask = masks.get(dest_ip, '')
		route.protocol = protocols.get(dest_ip, '')
		route.metric = metrics.get(dest_ip, '')
		route.ifindex = indexes.get(dest_ip, '')
		
		query.device.routes.append(route)
		
# key = [ipCidrRouteDest][ipCidrRouteMask][ipCidrRouteTos][ipCidrRouteNextHop]
# IP-FORWARD-MIB::ipCidrRouteDest[17.11.12.0][255.255.255.0][0][0.0.0.0] 17.11.12.0
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
	nexts = get_values4(contents, "ipCidrRouteNextHop")
	metrics = get_values4(contents, "ipCidrRouteMetric1")
	protocols = get_values4(contents, "ipCidrRouteProto")
	masks = get_values4(contents, "ipCidrRouteMask")
	indexes = get_values4(contents, "ipCidrRouteIfIndex")
	status = get_values4(contents, "ipCidrRouteStatus")
	for key in nexts.keys():
		if status.get(key, '') == 'active':
			route = Route()
			route.via_ip = nexts.get(key, '')
			route.dst_subnet = key[0]
			route.dst_mask = masks.get(key, '')
			route.protocol = protocols.get(key, '')
			route.metric = metrics.get(key, '')		# TODO: need to use other metrics if this one is -1
			route.ifindex = indexes.get(key, '')
			
			query.device.routes.append(route)
		
# key = [ifIndex]
# IF-MIB::ifIndex[1] 1
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
	descs = get_values1(contents, "ifDescr")
	macs = get_values1(contents, "ifPhysAddress")
	speeds = get_values1(contents, "ifSpeed")
	mtus = get_values1(contents, "ifMtu")
	in_octets = get_values1(contents, "ifInOctets")
	out_octets = get_values1(contents, "ifOutOctets")
	status = get_values1(contents, "ifOperStatus")
	last_changes = get_values1(contents, "ifLastChange")
	found = set()
	for index in descs.keys():
		# This is all kinds of screwed up but when devices are brought up and down multiple
		# entries land in the table. So what we'll do is add the ones that are enabled and
		# then add any that we missed that are down.
		if status.get(index, '') == 'up' or status.get(index, '') == 'dormant':
			name = descs.get(index, '').replace('/', '-')		# Ciscos use ifnames like FastEthernet0/0.8 which can cause problems with file paths and urls
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
	admin_status = get_values1(contents, "ifAdminStatus")
	oper_status = get_values1(contents, "ifOperStatus")
	for (index, admin) in admin_status.items():
		name = descs.get(index, '?')
		key = '%s-oper-status' % name
		if index in oper_status and admin != oper_status[index] and oper_status[index] != 'dormant':
			mesg = 'Admin set %s to %s but it is %s.' % (name, admin, oper_status[index])
			open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'error')	# TODO: what about resolution?
		else:
			close_alert(data, target, key)

# key = [ifindex?]
# IF-MIB::ifName[1] Fa0/0
# IF-MIB::ifInMulticastPkts[1] 96541
# IF-MIB::ifInBroadcastPkts[1] 51288
# IF-MIB::ifOutMulticastPkts[1] 3316182
# IF-MIB::ifOutBroadcastPkts[1] 980172
# IF-MIB::ifHCInOctets[1] 936496540
# IF-MIB::ifHCInUcastPkts[1] 3314972
# IF-MIB::ifHCInMulticastPkts[1] 96541
# IF-MIB::ifHCInBroadcastPkts[1] 51288
# IF-MIB::ifHCOutOctets[1] 1104561174
# IF-MIB::ifHCOutUcastPkts[1] 2626537
# IF-MIB::ifHCOutUcastPkts[20] 167792
# IF-MIB::ifHCOutMulticastPkts[1] 3316182
# IF-MIB::ifHCOutBroadcastPkts[1] 980172
# IF-MIB::ifLinkUpDownTrapEnable[1] enabled
# IF-MIB::ifHighSpeed[1] 100
# IF-MIB::ifPromiscuousMode[1] false
# IF-MIB::ifConnectorPresent[1] true
# IF-MIB::ifAlias[1] control interface
# IF-MIB::ifCounterDiscontinuityTime[1] 0
def process_ifX(data, contents, query):
	aliaii = get_values1(contents, "ifAlias")
	for (ifindex, alias) in aliaii.items():
		interface = find_interface(query.device, ifindex)
		interface.alias = alias

# HOST-RESOURCES-MIB::hrMemorySize.0 246004
#
# key = [hrStorageIndex]
# HOST-RESOURCES-MIB::hrStorageIndex[1] 1													one of these for each storage type
# HOST-RESOURCES-MIB::hrStorageType[1] HOST-RESOURCES-TYPES::hrStorageRam 	or hrStorageVirtualMemory, hrStorageOther, hrStorageFixedDisk
# HOST-RESOURCES-MIB::hrStorageDescr[1] Physical memory 									or Virtual memory, Memory buffers, Cached memory, Swap space, /rom, /overlay
# HOST-RESOURCES-MIB::hrStorageAllocationUnits[1] 1024
# HOST-RESOURCES-MIB::hrStorageSize[1] 246004
# HOST-RESOURCES-MIB::hrStorageUsed[1] 177396
def process_storage(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	storage = get_values1(contents, "hrStorageDescr")
	used = get_values1(contents, "hrStorageUsed")
	size = get_values1(contents, "hrStorageSize")
	units = get_values1(contents, "hrStorageAllocationUnits")
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

# key = [ciscoFlashDeviceIndex][ciscoFlashPartitionIndex]
# CISCO-FLASH-MIB::ciscoFlashPartitionName[1][1] flash:1
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

# key = [ciscoEnvMonTemperatureStatusIndex]
#  CISCO-ENVMON-MIB::ciscoEnvMonTemperatureStatusDescr[1] chassis
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureStatusValue[1] 23
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureThreshold[1] 65
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureState[1] normal		or warning, critical, shutdown, notPresent, notFunctioning
# CISCO-ENVMON-MIB::ciscoEnvMonTemperatureLastShutdown[1] 0
def process_temp(data, contents, query):
	#dump_snmp(query.device.admin_ip, 'temp', contents)
	target = 'entities:%s' % query.device.admin_ip
	names = get_values1(contents, "ciscoEnvMonTemperatureStatusDescr")
	status = get_values1(contents, "ciscoEnvMonTemperatureStatusValue")
	threshold = get_values1(contents, "ciscoEnvMonTemperatureThreshold")
	states = get_values1(contents, "ciscoEnvMonTemperatureState")
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

# key = [ciscoEnvMonFanStatusIndex]
# CISCO-ENVMON-MIB::ciscoEnvMonFanStatusDescr[1] Fan 1
# CISCO-ENVMON-MIB::ciscoEnvMonFanState[1] normal			or warning, critical, shutdown, notPresent, notFunctioning
def process_fan(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	names = get_values1(contents, "ciscoEnvMonFanStatusDescr")
	states = get_values1(contents, "ciscoEnvMonFanState")
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

# key = [cpmCPUTotalIndex]
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
	cpu = get_values1(contents, "cpmCPUTotal1minRev")
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

# key = [entPhysicalIndex]
# CISCO-ENTITY-EXT-MIB::ceExtProcessorRam[3] 268435456
# CISCO-ENTITY-EXT-MIB::ceExtNVRAMSize[3] 245752
# CISCO-ENTITY-EXT-MIB::ceExtNVRAMUsed[3] 42847
def process_nvram(data, contents, query):
	target = 'entities:%s' % query.device.admin_ip
	sizes = get_values1(contents, "ceExtNVRAMSize")
	used = get_values1(contents, "ceExtNVRAMUsed")
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

# key = [entPhysicalIndex][cempMemPoolIndex]
# CISCO-ENHANCED-MEMPOOL-MIB::cempMemPoolType[1][1] processorMemory
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
	names = get_values2(contents, "cempMemPoolType")
	useds = get_values2(contents, "cempMemPoolUsed")
	frees = get_values2(contents, "cempMemPoolFree")
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

# key = [ospfLsdbAreaId][ospfLsdbType][ospfLsdbLsid][ospfLsdbRouterId]
# OSPF-MIB::ospfLsdbAreaId[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 0.0.0.0  
# OSPF-MIB::ospfLsdbType[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] routerLink   or asExternalLink
# OSPF-MIB::ospfLsdbLsid[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 172.20.254.10
# OSPF-MIB::ospfLsdbRouterId[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 172.20.254.10
# OSPF-MIB::ospfLsdbSequence[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] -2147481898
# OSPF-MIB::ospfLsdbAge[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 1398
# OSPF-MIB::ospfLsdbChecksum[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] 11713
# OSPF-MIB::ospfLsdbAdvertisement[0.0.0.0][routerLink][172.20.254.10][172.20.254.10] "00 00..."
def process_ospf_lsdb(data, contents, query):
	#dump_snmp(query.device.admin_ip, 'ospf', contents)
	areas = get_values4(contents, "ospfLsdbAreaId")			# we don't actually use this, but have seen Ciscos where this is the only one available
	ages = get_values4(contents, "ospfLsdbAge")
	for (key, area) in areas.items():
		(area_id, kind, peer_ip, router_id) = key
		
		link = Link()
		link.admin_ip = query.device.admin_ip
		link.predicate = "options.ospf selection.name 'map' == and"
		link.peer_ip = peer_ip
		link.label1 = kind
		age = ages.get(key, '')
		if age:
			link.label1 = "%s old" % secs_to_str(int(age))
		
		# Lots of other types are possible but we really only want to show links to routers.
		if kind == 'routerLink':
			query.device.links.append(link)
	
# key = [ospfIfIpAddress][ospfAddressLessIf]
# OSPF-MIB::ospfIfIpAddress[17.11.12.12][0] 17.11.12.12
# OSPF-MIB::ospfAddressLessIf[17.11.12.12][0] 0
# OSPF-MIB::ospfIfAreaId[17.11.12.12][0] 0.0.0.0
# OSPF-MIB::ospfIfType[17.11.12.12][0] pointToPoint
# OSPF-MIB::ospfIfAdminStat[17.11.12.12][0] enabled
# OSPF-MIB::ospfIfRtrPriority[17.11.12.12][0] 0
# OSPF-MIB::ospfIfTransitDelay[17.11.12.12][0] 1
# OSPF-MIB::ospfIfRetransInterval[17.11.12.12][0] 5
# OSPF-MIB::ospfIfHelloInterval[17.11.12.12][0] 10
# OSPF-MIB::ospfIfRtrDeadInterval[17.11.12.12][0] 30
# OSPF-MIB::ospfIfPollInterval[17.11.12.12][0] 120
# OSPF-MIB::ospfIfState[17.11.12.12][0] pointToPoint
# OSPF-MIB::ospfIfDesignatedRouter[17.11.12.12][0] 0.0.0.0
# OSPF-MIB::ospfIfBackupDesignatedRouter[17.11.12.12][0] 0.0.0.0
# OSPF-MIB::ospfIfEvents[17.11.12.12][0] 1
# OSPF-MIB::ospfIfAuthKey[17.11.12.12][0] ""
# OSPF-MIB::ospfIfStatus[17.11.12.12][0] active
# OSPF-MIB::ospfIfMulticastForwarding[17.11.12.12][0] blocked
# OSPF-MIB::ospfIfDemand[17.11.12.12][0] false
# OSPF-MIB::ospfIfAuthType[17.11.12.12][0] none
def process_ospf_interfaces(data, contents, query):
	statuses = get_values2(contents, "ospfIfStatus")
	hellos = get_values2(contents, "ospfIfHelloInterval")
	deads = get_values2(contents, "ospfIfRtrDeadInterval")
	for (key, status) in statuses.items():
		if status == 'active':
			addr = key[0]
			value = hellos.get(key, None)
			if value:
				query.device.ospf_hellos[addr] = value
			value = deads.get(key, None)
			if value:
				query.device.ospf_deads[addr] = value

# key = [ipMRouteGroup][ipMRouteSource][ipMRouteSourceMask]
# IPMROUTE-STD-MIB::ipMRouteUpstreamNeighbor[226.3.1.0][172.20.18.10][255.255.255.255] 0.0.0.0
# IPMROUTE-STD-MIB::ipMRouteInIfIndex[226.3.1.0][172.20.18.10][255.255.255.255] 5
# IPMROUTE-STD-MIB::ipMRouteUpTime[226.3.1.0][172.20.18.10][255.255.255.255] 209292
# IPMROUTE-STD-MIB::ipMRouteExpiryTime[226.3.1.0][172.20.18.10][255.255.255.255] 12130
# IPMROUTE-STD-MIB::ipMRoutePkts[226.3.1.0][172.20.18.10][255.255.255.255] 4186
# IPMROUTE-STD-MIB::ipMRouteDifferentInIfPackets[226.3.1.0][172.20.18.10][255.255.255.255] 0
# IPMROUTE-STD-MIB::ipMRouteOctets[226.3.1.0][172.20.18.10][255.255.255.255] 452088
# IPMROUTE-STD-MIB::ipMRouteProtocol[226.3.1.0][172.20.18.10][255.255.255.255] pimSparseMode
# IPMROUTE-STD-MIB::ipMRouteRtProto[226.3.1.0][172.20.18.10][255.255.255.255] local
# IPMROUTE-STD-MIB::ipMRouteRtAddress[226.3.1.0][172.20.18.10][255.255.255.255] 172.20.18.0
# IPMROUTE-STD-MIB::ipMRouteRtMask[226.3.1.0][172.20.18.10][255.255.255.255] 255.255.255.224
# IPMROUTE-STD-MIB::ipMRouteRtType[226.3.1.0][172.20.18.10][255.255.255.255] multicast
# IPMROUTE-STD-MIB::ipMRouteHCOctets[226.3.1.0][172.20.18.10][255.255.255.255] 452088
def process_mroute(data, contents, query):
	#dump_snmp(query.device.admin_ip, 'mroute', contents)
	upstreams = get_values3(contents, "ipMRouteUpstreamNeighbor")
	uptimes = get_values3(contents, "ipMRouteUpTime")
	num_packets = get_values3(contents, "ipMRoutePkts")
	num_octets = get_values3(contents, "ipMRouteOctets")
	protocols = get_values3(contents, "ipMRouteProtocol")
	for (key, upstream_ip) in upstreams.items():
		(group, source, source_netmask) = key
		route = MRoute()
		route.admin_ip = query.device.admin_ip
		route.group = group
		route.source = source
		route.upstream = upstream_ip
		age = uptimes.get(key, '')
		if age:
			route.uptime = secs_to_str(float(age)/100)
			route.label1 = route.uptime + " old"
		protocol = protocols.get(key, '')
		if protocol.endswith('Mode'):
			route.protocol = protocol[:-len('Mode')]
			route.label2 = route.protocol
		pkts = num_packets.get(key, '')
		if pkts:
			route.packets = float(pkts)
			route.label3 = "%spkt" % to_si(route.packets)
		octets = num_octets.get(key, '')
		if octets:
			route.octets = float(octets)
		
		# TODO: This kind of sucks because we're not showing what the upstream routers do
		# but what a downstream router would do if it got packets. But afaict there is no way
		# to get out interfaces for multicast using snmp.
		query.device.mroutes.append(route)

# key = [igmpCacheAddress][igmpCacheIfIndex]
# IGMP-STD-MIB::igmpCacheSelf[226.3.1.0][6] false
# IGMP-STD-MIB::igmpCacheLastReporter[226.3.1.0][6] 172.20.19.10
# IGMP-STD-MIB::igmpCacheUpTime[226.3.1.0][6] 551600
# IGMP-STD-MIB::igmpCacheExpiryTime[226.3.1.0][6] 12200
# IGMP-STD-MIB::igmpCacheStatus[226.3.1.0][6] active
def process_igmp(data, contents, query):
	#dump_snmp(query.device.admin_ip, 'igmp', contents)
	reporters = get_values2(contents, "igmpCacheLastReporter")
	uptimes = get_values2(contents, "igmpCacheUpTime")
	statuses = get_values2(contents, "igmpCacheStatus")
	for (key, reporter_ip) in reporters.items():
		(group, ifindex) = key
		igmp = Igmp()
		igmp.group = group
		igmp.reporter = reporter_ip
		igmp.status = statuses.get(key, '')
		age = uptimes.get(key, '')
		if age:
			igmp.uptime = float(age)/100
		
		query.device.igmps.append(igmp)

# PIM-MIB::pimJoinPruneInterval.0 60
# 
# key = [pimNeighborAddress]
# PIM-MIB::pimNeighborIfIndex[17.11.12.11] 14
# PIM-MIB::pimNeighborUpTime[17.11.12.11] 71287
# PIM-MIB::pimNeighborExpiryTime[17.11.12.11] 2850
# PIM-MIB::pimNeighborMode[17.11.12.11] dense
# 
# key = [pimInterfaceIfIndex]
# PIM-MIB::pimInterfaceAddress[6] 172.20.12.62
# PIM-MIB::pimInterfaceMode[6] dense
# PIM-MIB::pimInterfaceDR[6] 172.20.12.62
# PIM-MIB::pimInterfaceHelloInterval[6] 10
# PIM-MIB::pimInterfaceStatus[6] active
# PIM-MIB::pimInterfaceJoinPruneInterval[6] 60
# PIM-MIB::pimInterfaceCBSRPreference[6] -1
def process_pim(data, contents, query):
	ages = get_values1(contents, "pimNeighborUpTime")	# note that add_link_relations will automagically add left and right labels with interface info
	models = get_values1(contents, "pimNeighborMode")
	for (peer_ip, model) in models.items():
		link = Link()
		link.admin_ip = query.device.admin_ip
		link.predicate = "options.pim"
		link.peer_ip = peer_ip
		age = ages.get(peer_ip, '')
		if age:
			link.label1 = "%s old" % secs_to_str(int(age)/100.0)
		link.label2 = model
		
		query.device.links.append(link)
	
	hellos = get_values1(contents, "pimInterfaceHelloInterval")
	statuses = get_values1(contents, "pimInterfaceStatus")
	for (ifindex, hello) in hellos.items():
		if statuses.get(ifindex, '') == 'active':
			query.device.pim_hellos[ifindex] = hello
	
# key = [pimRPSetComponent][pimRPSetGroupAddress][pimRPSetGroupMask][pimRPSetAddress]
# PIM-MIB::pimRPSetHoldTime[1][224.0.0.0][240.0.0.0][172.20.12.113] 0
# PIM-MIB::pimRPSetExpiryTime[1][224.0.0.0][240.0.0.0][172.20.12.113] 0
def process_pim_rp(data, contents, query):
	times = get_values4(contents, "pimRPSetHoldTime")
	for (key, time) in times.items():
		(component, group, mask, rp) = key
		query.device.pim_rps.append(rp)
	
# key = [pimComponentIndex]
# PIM-MIB::pimComponentBSRAddress[1] 172.20.12.113
# PIM-MIB::pimComponentBSRExpiryTime[1] 12202
# PIM-MIB::pimComponentCRPHoldTime[1] 0
# PIM-MIB::pimComponentStatus[1] active
def process_pim_component(data, contents, query):
	bsrs = get_values1(contents, "pimComponentBSRAddress")
	statuses = get_values1(contents, "pimComponentStatus")
	for (component, bsr) in bsrs.items():
		if statuses.get(component, '') == 'active':
			query.device.pim_bsrs.append(bsr)
	
# key = [hrDeviceIndex ]
# HOST-RESOURCES-MIB::hrDeviceIndex[768] 768
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
	descrs = get_values1(contents, "hrDeviceDescr")
	status = get_values1(contents, "hrDeviceStatus")
	errors = get_values1(contents, "hrDeviceErrors")
	for (index, desc) in descrs.items():
		# update system details with info about devices
		stat = status.get(index, '')
		errs = errors.get(index, '0')
		if stat:
			query.device.system_info += '* %s is %s with %s errors\n' % (desc, stat, errs)
		
	# add a gauge if processor load is high
	loads = get_values1(contents, 'hrProcessorLoad')
	for (index, load) in loads.items():
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
			add_gauge(data, target, 'processor %s load' % index, value, level, style, sort_key = 'y')
			
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

# Returns a dict mapping the key to values.
# HOST-RESOURCES-MIB::hrFSIndex[1] 1
def get_values1(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[ ([^\]]+) \] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		value = match.group(2)
		if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
			value = value[1:-1]
		values[match.group(1)] = value
	
	return values

# Returns a dict mapping [key1, key2] to values.
# CISCO-FLASH-MIB::ciscoFlashPartitionFileCount[1][1] 29
def get_values2(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[ ([^\]]+) \] \[ ([^\]]+) \] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		values[(match.group(1), match.group(2))] = match.group(3)
	
	return values

# Returns a dict mapping [key1, key2, key3] to values.
# IPMROUTE-STD-MIB::ipMRouteInIfIndex[226.3.1.0][172.20.18.10][255.255.255.255] 16
def get_values3(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[ ([^\]]+) \] \[ ([^\]]+) \] \[ ([^\]]+) \] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		values[(match.group(1), match.group(2), match.group(3))] = match.group(4)
	
	return values

# Returns a dict mapping [key1, key2, key3, key4] to values.
# IP-FORWARD-MIB::ipCidrRouteIfIndex[17.11.12.0][255.255.255.0][0][0.0.0.0] 24
def get_values4(contents, name):
	values = {}
	
	expr = re.compile(r'::%s \[ ([^\]]+) \] \[ ([^\]]+) \] \[ ([^\]]+) \] \[ ([^\]]+) \] \  (.+)$' % name, re.MULTILINE | re.VERBOSE)
	for match in re.finditer(expr, contents):
		values[(match.group(1), match.group(2), match.group(3), match.group(4))] = match.group(5)
	
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
		self.__mibs = ['sysUpTime sysUpTimeInstance sysDescr sysContact ipForwarding', 'ipAddrTable', 'interfaces', 'ifXTable']
		for mib in device.config.get('mibs', '').split(' '):
			if mib == 'cisco-router':
				add_if_missing(self.__mibs, 'ciscoFlashPartitionTable')
				add_if_missing(self.__mibs, 'ciscoEnvMonTemperatureStatusTable')
				add_if_missing(self.__mibs, 'ciscoEnvMonFanStatusTable')
				add_if_missing(self.__mibs, 'cpmCPUTotalTable')
				add_if_missing(self.__mibs, 'ceExtPhysicalProcessorTable')
				add_if_missing(self.__mibs, 'cempMemPoolTable')
				add_if_missing(self.__mibs, 'ipCidrRouteTable')
				add_if_missing(self.__mibs, 'ospfLsdbTable')
				add_if_missing(self.__mibs, 'ospfIfTable')
				add_if_missing(self.__mibs, 'ipMRouteTable')
				add_if_missing(self.__mibs, 'pim')
				add_if_missing(self.__mibs, 'pimRPSetTable')
				add_if_missing(self.__mibs, 'pimComponentTable')
				add_if_missing(self.__mibs, 'igmpCacheTable')
			elif mib == 'linux-router':
				add_if_missing(self.__mibs, 'ipRouteTable')		# TODO: probably want to add ospf and pim mibs
				add_if_missing(self.__mibs, 'hrStorageTable')
				add_if_missing(self.__mibs, 'hrDevice')
				add_if_missing(self.__mibs, 'igmpCacheTable')
			elif  mib == 'linux-host':
				add_if_missing(self.__mibs, 'ipRouteTable')
				add_if_missing(self.__mibs, 'hrStorageTable')
				add_if_missing(self.__mibs, 'hrDevice')
			else:
				if mib in self.__handlers:
					add_if_missing(self.__mibs, mib)
				else:
					env.logger.error("Don't know how to parse %s mib" % mib)
		self.__handlers = {
			'sysUpTime sysUpTimeInstance sysDescr sysContact ipForwarding': process_misc,
			'ipAddrTable': process_ip_addr,
			'ipRouteTable': process_ip_route,
			'ipCidrRouteTable': process_ip_cidr,
			'interfaces': process_interfaces,
			'hrStorageTable': process_storage,
			'hrDevice': process_device,
			'ciscoFlashPartitionTable': process_flash,
			'ciscoEnvMonTemperatureStatusTable': process_temp,
			'ciscoEnvMonFanStatusTable': process_fan,
			'cpmCPUTotalTable': process_cpu,
			'ceExtPhysicalProcessorTable': process_nvram,
			'cempMemPoolTable': process_mempool,
			'ospfLsdbTable': process_ospf_lsdb,
			'ospfIfTable': process_ospf_interfaces,
			'ipMRouteTable': process_mroute,
			'pim': process_pim,
			'pimRPSetTable': process_pim_rp,
			'pimComponentTable': process_pim_component,
			'ifXTable': process_ifX,
			'igmpCacheTable': process_igmp,
		}
		self.device = device
	
	def run(self):
		self.__results = {}
		try:
			for name in self.__mibs:
				if ' ' in name:
					# Note that snmpbulkget does not return all the results for ipAddrTable or (I think) ipCidrRouteTable.
					# As far as I can tell there is a fairly small max size for the amount of data a GET will return (no
					# more than a single packet). Could do individual GETs for each value...
					command = 'snmpbulkget %s %s -Oq -Ot -OU -OX %s' % (self.device.config['authentication'], self.device.admin_ip, name)
				else:
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
