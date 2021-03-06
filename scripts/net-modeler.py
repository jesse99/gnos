#!/usr/bin/python
# Collects information about a network using snmp and ssh. Ships the results off to gnos using json.
import cgi, httplib, json, socket, sys, threading, time
import linux_ssh, snmp
from helpers import *
from net_types import *

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)

def add_units(value, unit):
	if value or type(value) == float:
		value = '%s %s' % (value, unit)
	return value

# It's a little lame that the edges have to be specified in the network file (using
# the links list) but relations don't work so well as edges because there are
# often too many of them (which causes clutter and, even worse, causes a
# lot of instability in node positions when there are too many forces acting
# on nodes (even with very high friction levels)).
def send_entities(connection):
	def find_ip(name):
		for (candidate, device) in env.config["devices"].items():
			if candidate == name:
				return device['ip']
		env.logger.error("Couldn't find link to %s" % name)
		return ''
		
	entities = []
	relations = []
	for (name, device) in env.config["devices"].items():
		style = "font-weight:bolder"
		if '-host' in device['mibs']:		# TODO: probably should have a dedicated device type field
			style += ' frame-back-color:lavender'
		entity = {"id": device['ip'], "label": name, "style": style}
		env.logger.debug("entity: %s" % entity)
		entities.append(entity)
		
		if 'links' in device:
			for link in device['links']:
				ip = find_ip(link)
				if ip:
					left = 'entities:%s' % device['ip']
					right = 'entities:%s' % ip
					relation = {'left-entity-id': left, 'right-entity-id': right, 'predicate': 'options.none'}
					relations.append(relation)
	send_update(connection, {"modeler": "config", "entities": entities, 'relations': relations})
	
def mask_to_subnet(s):
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
	
	if s:
		parts = s.split('.')
		bytes = map(lambda p: int(p), parts)
		mask = reduce(lambda sum, current: 256*sum + current, bytes, 0)
		leading = count_leading_ones(mask)
		trailing = count_trailing_zeros(mask)
		if leading + trailing == 32:
			return leading
		else:
			return s		# unusual netmask where 0s and 1s are mixed.
	else:
		'?'

class SnmpThread(threading.Thread):
	def __init__(self, device, queriers, check_queries):
		threading.Thread.__init__(self)
		self.__query = snmp.QueryDevice(device)
		self.__queriers = queriers
		self.__check_queries = check_queries
		
	def run(self):
		self.__query.run()
		
		self.__check_queries.acquire()
		try:
			self.__queriers.append(self.__query)
			self.__check_queries.notify()
		finally:
			self.__check_queries.release()
		
class SshThread(threading.Thread):
	def __init__(self, queriers, check_queries):
		threading.Thread.__init__(self)
		self.__queries = []
		self.__queriers = queriers
		self.__check_queries = check_queries
		
	@property
	def num_devices(self):
		return len(self.__queries)
		
	def add_device(self, device):
		self.__queries.append(linux_ssh.QueryDevice(device))
		
	def run(self):
		for query in self.__queries:
			query.run()
		
		self.__check_queries.acquire()
		try:
			self.__queriers.extend(self.__queries)
			self.__check_queries.notify()
		finally:
			self.__check_queries.release()
		
class Poll(object):
	def __init__(self):
		self.__startTime = time.time()
		self.__last_time = None
		self.__num_updates = 0
		self.__last_sample = {}
		self.__connection = None
		if env.options.put:
			address = "%s:%s" % (env.config['server'], env.config['port'])
			self.__connection = httplib.HTTPConnection(address, strict = True, timeout = 10)
	
	# Note that this processing takes a neglible amount of time (less than a tenth of a second).
	# All the time is spent in run_process waiting for the results of the snmp commands (together
	# these take 5s for a network with 6 cisco routers and 8 linux hosts).
	def run(self):
		try:
			if self.__connection:
				send_entities(self.__connection)
			
			rate = env.config['poll-rate']
			while True:
				self.__current_time = time.time()
				if not env.options.put:
					env.logger.info("-" * 60)
					
				devices = map(lambda e: Device(e[0], e[1]), env.config['devices'].items())
				data = {'modeler': 'net', 'entities': [], 'relations': [], 'labels': [], 'gauges': [], 'details': [], 'alerts': [], 'samples': [], 'charts': []}
				self.__query_devices(data, devices)
				self.__update_routes(devices)
				
				self.__set_pim_routers(devices)
				for device in devices:
					self.__process_device(data, devices, device)
				self.__add_next_hop_relations(data, devices)
				self.__add_selection_route_relations(data, devices)
				self.__add_network_ips(data, devices)
				self.__add_network_mroutes(data, devices)
				self.__add_network_pim(data, devices)
				self.__add_network_ospf(data, devices)
				self.__add_mroutes(data, devices)
				self.__add_igmp(data, devices)
				self.__add_link_relations(data, devices)
				if self.__num_updates >= 2:
					self.__add_bandwidth_details(data, 'out')
					self.__add_bandwidth_details(data, 'in')
				
				send_update(self.__connection, data)
				self.__num_updates += 1
				
				elapsed = time.time() - self.__current_time
				self.__last_time = self.__current_time
				env.logger.info('elapsed: %.1f seconds' % elapsed)
				if time.time() - self.__startTime < env.options.duration:
					time.sleep(max(rate - elapsed, 5))
				else:
					break
		finally:
			if self.__connection:
				self.__connection.close()
				
	def __query_devices(self, data, devices):
		threads = []
		queriers = []
		num_queries = 0
		check_queries = threading.Condition(threading.Lock())
		
		# Query the devices using a thread to get the raw data and possibly
		# update device.
		ssh = SshThread(queriers, check_queries)
		for device in devices:
			if device.config['type'] == 'snmp':
				thread = SnmpThread(device, queriers, check_queries)
				thread.start()
				threads.append(thread)
				num_queries += 1
			elif device.config['type'] == 'linux_ssh':
				ssh.add_device(device)
				num_queries += 1
			else:
				env.logger.error("bad modeler: %s" % device.config['modeler'])
				
		# For some reason when we try to ssh using multiple threads the results are
		# empty for all or (sometimes) all but one. So we'll do all of them within a
		# single thread.
		if ssh.num_devices > 0:
			ssh.start()
			threads.append(ssh)
			
		# But the data argument is a shared resource so only update that from one
		# thread.
		count = 0
		while count < num_queries:
			check_queries.acquire()
			try:
				while len(queriers) == 0:
					check_queries.wait(max(5.0, 2.0*ssh.num_devices))
				while len(queriers) > 0:
					queriers.pop().process(data)
					count += 1
			except RuntimeError:
				env.logger.error("Failed to get data for a device (probably timed out)", exc_info=True)	# i.e. wait timed out
				count = len(threads)
			finally:
				check_queries.release()
		
	def __update_routes(self, devices):
		def interface_by_index(device, ifindex):
			for candidate in device.interfaces:
				if candidate.index == ifindex:
					return candidate
			return None
			
		def interface_by_device_ip(devices, ip):
			for device in devices:
				for interface in device.interfaces:
					if interface.ip == ip:
						return interface
			return None
			
		for device in devices:
			for route in device.routes:
				route.src_interface = interface_by_index(device, route.ifindex)
				if not route.src_interface:
					env.logger.	warning("Couldn't find an interface for route %s on %s" % (route, device.admin_ip))
				if route.via_ip != None and route.via_ip != '0.0.0.0':
					route.via_interface = interface_by_device_ip(devices, route.via_ip)
				if route.dst_subnet and route.dst_mask:
					route.dst_admin_ips = self.admin_ips_for_route(devices, device, route)
		
	def admin_ips_for_route(self, devices, in_device, route):
		ips = []
		
		# 1) If the dst mask is all 1s then the route is to a single machine.
		if route.dst_mask == '255.255.255.255':
			ips.append(self.admin_ip_by_device_ip(devices, route.dst_subnet))
			
		# 2) If the dst mask is a subnet then the route is to all devices in that subnet.
		elif route.dst_mask != '0.0.0.0':
			netmask = ip_to_int(route.dst_mask)
			subnet = ip_to_int(route.dst_subnet) & netmask
			for candidate in devices:
				if candidate.admin_ip != in_device.admin_ip:
					candidate_ips = [i.ip for i in candidate.interfaces if i.ip and (ip_to_int(i.ip) & netmask) == subnet]
					if candidate_ips:
						ips.append(candidate.admin_ip)
			
		# 2) If the dst mask is all 0s then the route is to every device
		else:
			for candidate in devices:
				if candidate.admin_ip != in_device.admin_ip:
					ips.append(candidate.admin_ip)
		
		return ips
		
	def admin_ip_by_device_ip(self, devices, ip):
		# See if the ip is already an admin ip (we do this to ensure that we can
		# find an admin ip even for downed devices).
		for device in env.config["devices"].values():
			if device['ip'] == ip:
				return ip
		
		# Otherwise try and find a device with that ip.
		for device in devices:
			for interface in device.interfaces:
				if interface.ip == ip:
					return device.admin_ip
				
		return None
		
	def device_name(self, devices, src_device, ip, flags = ''):
		# First try to get the actual name.
		for (name, device) in env.config["devices"].items():
			if device['ip'] == ip:
				return name
		
		for device in devices:
			for interface in device.interfaces:
				if interface.ip == ip:
					return device.name
					
		if flags == 'name-only':
			return None
		
		# Then try to get the admin ip
		admin_ip = self.admin_ip_by_device_ip(devices, ip)
		if admin_ip:
			return admin_ip
		
		# As a last result just return the ip.
		return ip
		
	# Returns admin ips of the devices connected to device (via json network file).
	def __find_config_link_ips(self, device):
		ips = []
		
		for (name, candidate) in env.config["devices"].items():
			if name != device.name:
				if device.name in candidate.get('links', []):
					ips.append(candidate['ip'])
			else:
				for n in candidate.get('links', []):
					d = env.config["devices"][n]
					if 'ip' in d:
						ips.append(d['ip'])
					else:
						env.logger.error("%s has no ip key" % d)
		
		return ips
			
	def __set_pim_routers(self, devices):
		rps = set()
		bsrs = set()
		for device in devices:
			rps.update(device.pim_rps)
			bsrs.update(device.pim_bsrs)
		
		for device in devices:
			for rp in rps:
				if device.find_ip(rp):
					device.is_rp = True
					break
			
			for bsr in bsrs:
				if device.find_ip(bsr):
					device.is_bsr = True
					break

	def __process_device(self, data, devices, device):
		# admin ip label
		target = 'entities:%s' % device.admin_ip
		add_label(data, target, device.admin_ip, 'a', level = 1, style = 'font-size:x-small')
		
		if device.uptime:
			# pim RP and BSR
			if device.is_rp and device.is_bsr:
				add_label(data, target, 'RP & BSR', 'aaa', level = 1, style = 'font-size:x-small font-weight:bolder')
			elif device.is_rp:
				add_label(data, target, 'RP', 'aaa', level = 1, style = 'font-size:x-small font-weight:bolder')
			elif device.is_bsr:
				add_label(data, target, 'BSR', 'aaa', level = 1, style = 'font-size:x-small font-weight:bolder')
			
			# uptime label
			key = 'alpha'		# want these to appear before most other labels
			add_label(data, target, 'uptime: %s' % secs_to_str(device.uptime), key, level = 2, style = 'font-size:x-small')
			
			# uptime alert
			if device.uptime < 60.0:
				# TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
				open_alert(data, target, key = 'uptime', mesg = 'Device rebooted.', resolution = '', kind = 'error')
			else:
				close_alert(data, target, key = 'uptime')
				
		# system info
		if device.system_info:
			add_details(data, target, 'System Info', [device.system_info], opened = 'no', sort_key = 'beta', key = 'system info')
			
		# misc
		if device.interfaces:
			self.__add_interfaces_table(data, device)
		if device.routes:
			self.__add_routing_table(data, devices, device)
		if self.__num_updates >= 2:
			self.__add_bandwidth_chart(data, 'out', device)
			self.__add_bandwidth_chart(data, 'in', device)
		self.__add_interface_uptime_alert(data, device)
			
	def __add_interface_uptime_alert(self, data, device):
		for interface in device.interfaces:
			if interface.last_changed:
				delta = device.uptime - interface.last_changed
				key = '%s-last-change' % interface.name
				target = 'entities:%s' % device.admin_ip
				if delta >= 0.0 and delta < 60.0:
					mesg = '%s status recently changed to %s.' % (interface.name, interface.status)
					open_alert(data, target, key, mesg = mesg, resolution = '', kind = 'warning') 
				else:
					close_alert(data, target, key)
	
	def __add_bandwidth_chart(self, data, direction, device):
		samples = []
		legends = []
		table = sorted(device.interfaces, key = lambda i: i.name)
		for interface in table:
			if interface.active:
				if (direction == 'in' and interface.in_octets != None) or (direction == 'out' and interface.out_octets != None):
					name = interface.name
					legends.append(name)
					samples.append('%s-%s-%s_octets' % (device.admin_ip, name, direction))
		
		if samples:
			name = "%s-%s_interfaces" % (device.admin_ip, direction)
			data['charts'].append({'admin_ip': device.admin_ip, 'direction': direction, 'name': name, 'samples': samples, 'legends': legends, 'title': '%s Bandwidth' % direction.title(), 'y_label': 'Bandwidth (kbps)'})
		
	def __add_bandwidth_details(self, data, direction):
		for chart in data['charts']:
			if chart['direction'] == direction:
				target = 'entities:%s' % chart['admin_ip']
				name = chart['name']
				markdown = '![bandwidth](/generated/%s.png#%s)' % (name, self.__num_updates)
				add_details(data, target, '%s Bandwidth' % direction.title(), [markdown], opened = 'no', sort_key = 'alpha-' + direction, key = '%s bandwidth' % name)
			
	def __add_routing_table(self, data, devices, device):
		rows = []
		for route in device.routes:
			dest = route.dst_subnet
			subnet = mask_to_subnet(route.dst_mask)
			dest = '%s/%s' % (dest, subnet)
			
			if len(route.dst_admin_ips) == 1:
				name = self.device_name(devices, device, route.dst_admin_ips[0], 'name-only')
				if name:
					dest = name + ' ' + dest
			
			if route.src_interface:
				out = route.src_interface.name
			else:
				out = ''
			
			via = None
			if route.via_ip != '0.0.0.0':
				name = self.device_name(devices, device, route.via_ip, 'name-only')
				if name:
					via = name + ' ' + route.via_ip
			if not via:
				if route.via_interface:
					via = route.via_interface.name + ' ' + route.via_ip
				elif route.via_ip != '0.0.0.0':
					via = route.via_ip
				else:
					via = ''
			
			rows.append([dest, via, out, route.protocol, route.metric])
			
		detail = {}
		detail['style'] = 'plain'
		detail['header'] = ['Destination', 'Via', 'Out', 'Protocol', 'Cost']
		detail['rows'] = sorted(rows, key = lambda row: row[0])
		
		target = 'entities:%s' % device.admin_ip
		add_details(data, target, 'Routes', [detail], opened = 'no', sort_key = 'beta', key = 'routing table')
	
	def __add_selection_route_relations(self, data, devices):
		routes = {}			# (src admin ip, via admin ip, dst admin ip) => Route
		for device in devices:
			for route in device.routes:
				# We're forwarding through a known waypoint.
				if route.via_interface:
					via_admin = route.via_interface.admin_ip
					for dst in route.dst_admin_ips:
						if dst != via_admin:
							key = (device.admin_ip, via_admin, dst)
							routes[key] = route
				
				# We're forwarding through a waypoint that we don't know about (could be something wacky 
				# like a radio acting as an L3 device that we aren't modeling). In this case we have no way
				# of knowing for sure what the next device is so we'll make the assumption that the links info
				# from the config is accurate.
				elif route.via_ip and route.via_ip != '0.0.0.0':
					admin_ips = self.__find_config_link_ips(device)
					if len(admin_ips) == 1:				# if there is one link we'll use it
						for dst in route.dst_admin_ips:
							if dst != admin_ips[0]:
								key = (device.admin_ip, admin_ips[0], dst)
								routes[key] = route
					else:
						old_len = len(routes)				# otherwise we'll use links matching the dst
						netmask = ip_to_int(route.dst_mask)
						subnet = ip_to_int(route.dst_subnet) & netmask
						for admin_ip in admin_ips:
							if ip_to_int(admin_ip) & netmask == subnet:
								key = (device.admin_ip, admin_ip, admin_ip)
								routes[key] = route
						if len(routes) == old_len:
							env.logger.error("Couldn't find dst %s via %s from %s", route.dst_subnet, route.via_ip, device.admin_ip)
						
				# We're forwarding directly to the destination (or destinations for routers connected to a LAN).
				else:
					for dst in route.dst_admin_ips:
						key = (device.admin_ip, dst, dst)
						routes[key] = route
		
		for (key, route) in routes.items():
			(src_admin, via_admin, dst_admin) = key
			left = 'entities:%s' % src_admin
			right = 'entities:%s' % via_admin
			
			left_labels = []
			if route.src_interface:
				left_labels.append({'label': route.src_interface.name, 'level': 2, 'style': 'font-size:xx-small'})
				left_labels.append({'label': route.src_interface.ip, 'level': 3, 'style': 'font-size:xxx-small'})
				if route.src_interface.mac_addr:
					left_labels.append({'label': route.src_interface.mac_addr, 'level': 4, 'style': 'font-size:xxx-small'})
			
			color = 'blue'
			middle_labels = []
			middle_labels.append({'label': '%s cost %s' % (route.protocol, route.metric), 'level': 1, 'style': 'font-size:x-small'})
			if not route.via_interface and route.via_ip and route.via_ip != '0.0.0.0':
				color = 'darkviolet'
				middle_labels.append({'label': "via isn't modeled", 'level': 4, 'style': 'font-size:x-small'})
			
			right_labels = []
			if route.via_interface:
				right_labels.append({'label': route.via_interface.name, 'level': 2, 'style': 'font-size:xx-small'})
			if route.via_ip != '0.0.0.0':
				right_labels.append({'label': route.via_ip, 'level': 3, 'style': 'font-size:xxx-small'})
			elif route.dst_subnet and route.dst_mask:
				subnet = mask_to_subnet(route.dst_mask)
				right_labels.append({'label': "%s/%s" % (route.dst_subnet, subnet), 'level': 3, 'style': 'font-size:xxx-small'})
			if route.via_interface and route.via_interface.mac_addr:
				right_labels.append({'label': route.via_interface.mac_addr, 'level': 4, 'style': 'font-size:xxx-small'})
			
			predicate = "options.ospf options.routes or selection.name '%s' ends_with and" % dst_admin
			add_relation(data, left, right, 'line-type:directed line-color:%s line-width:3' % color, left_labels = left_labels, middle_labels = middle_labels, right_labels = right_labels, predicate = predicate)
		
	def __add_link_relations(self, data, devices):
		links = {}			# (src admin ip, peer admin ip, predicate) => Link
		for device in devices:
			for link in device.links:
				if link.peer_ip:
					peer_admin_ip = self.admin_ip_by_device_ip(devices, link.peer_ip)
					if peer_admin_ip:
						links[(link.admin_ip, peer_admin_ip, link.predicate)] = link
					else:
						env.logger.error("Couldn't find link to %s on %s" % (link.peer_ip, link.admin_ip))
		
		for (key, link) in links.items():
			(src_admin, peer_admin, predicate) = key
			style = None
			left_labels = []
			if (peer_admin, src_admin, predicate) in links:
				if src_admin < peer_admin:
					style = 'line-type:bidirectional'
					left_labels.append({'label': links[(peer_admin, src_admin, predicate)].peer_ip, 'level': 3, 'style': 'font-size:xxx-small'})
			else:
				style = 'line-type:directed line-color:red'
			if style:
				left = 'entities:%s' % src_admin
				right = 'entities:%s' % peer_admin
				middle_labels = []
				if link.label1:
					middle_labels.append({'label': link.label1, 'level': 1, 'style': 'font-size:x-small'})
				if link.label2:
					middle_labels.append({'label': link.label2, 'level': 2, 'style': 'font-size:x-small'})
				if link.label3:
					middle_labels.append({'label': link.label3, 'level': 3, 'style': 'font-size:x-small'})
				right_labels = [{'label': link.peer_ip, 'level': 3, 'style': 'font-size:xxx-small'}]
				add_relation(data, left, right, style, left_labels = left_labels, middle_labels = middle_labels, right_labels = right_labels, predicate = predicate)
		
	def __add_next_hop_relations(self, data, devices):
		routes = {}			# (src admin ip, via admin ip) => Route
		for device in devices:
			for route in device.routes:
				if route.via_interface:
					src_admin = route.src_interface.admin_ip
					via_admin = route.via_interface.admin_ip
					if src_admin != via_admin:
						routes[(src_admin, via_admin)] = route
				elif route.src_interface:
					for dst in route.dst_admin_ips:
						src_admin = route.src_interface.admin_ip
						routes[(src_admin, dst)] = route
		
		for (key, route) in routes.items():
			(src_admin, via_admin) = key
			style = None
			if (via_admin, src_admin) in routes:
				if src_admin < via_admin:
					style = 'line-type:bidirectional'
			else:
				style = 'line-type:directed'
			if style:
				left = 'entities:%s' % src_admin
				right = 'entities:%s' % via_admin
				predicate = "options.routes selection.name 'map' == and"
				add_relation(data, left, right, style, middle_labels = [{'label': 'next hop', 'level': 1, 'style': 'font-size:x-small'}], predicate = predicate)
		
	def __add_mroutes(self, data, devices):
		routes = []
		for device in devices:
			for route in device.mroutes:
				if route.source != '0.0.0.0':
					name = '%s_from_%s' % (route.group, self.device_name(devices, device, route.source))
					predicate = "options.%s" % name
					style = 'line-type:directed'
					add_if_missing(routes, (route.group, route.source, name, predicate))
					
					# handle upstream from mroute
					if route.source != '0.0.0.0':
						up_admin_ip = self.admin_ip_by_device_ip(devices, route.upstream)
						if up_admin_ip:
							left = 'entities:%s' % up_admin_ip
							right = 'entities:%s' % route.admin_ip
							
							middle_labels = []
							if route.label1:
								middle_labels.append({'label': route.label1, 'level': 1, 'style': 'font-size:x-small'})
							if route.label2:
								middle_labels.append({'label': route.label2, 'level': 2, 'style': 'font-size:x-small'})
							if route.label3:
								middle_labels.append({'label': route.label3, 'level': 3, 'style': 'font-size:x-small'})
							add_relation(data, left, right, style, middle_labels = middle_labels, predicate = predicate)
							
					# handle downstream from igmp
					for igmp in device.igmps:
						if igmp.group == route.group:
							reporter_admin_ip = self.admin_ip_by_device_ip(devices, igmp.reporter)
							if reporter_admin_ip:
								left = 'entities:%s' % device.admin_ip
								right = 'entities:%s' % reporter_admin_ip
								
								middle_labels = []
								if igmp.age:
									middle_labels.append({'label': secs_to_str(igmp.age) + " old", 'level': 1, 'style': 'font-size:x-small'})
								middle_labels.append({'label': 'igmp', 'level': 2, 'style': 'font-size:x-small'})
								add_relation(data, left, right, style, middle_labels = middle_labels, predicate = predicate)
							
		# handle upstream from source
		for (group, source, name, predicate) in routes:
			for candidate in devices:
				if candidate.find_ip(source):
					vias = [r for r in candidate.routes if r.via_ip and r.via_ip != '0.0.0.0' and r.via_interface]
					for v in vias:
						left = 'entities:%s' % candidate.admin_ip
						right = 'entities:%s' % v.via_interface.admin_ip
						
						middle_labels = [{'label': 'gateway', 'level': 2, 'style': 'font-size:x-small'}]
						add_relation(data, left, right, 'line-type:directed', middle_labels = middle_labels, predicate = predicate)
					break
		
	def __add_igmp(self, data, devices):
		for device in devices:
			for igmp in device.igmps:
				reporter_admin_ip = self.admin_ip_by_device_ip(devices, igmp.reporter)
				if reporter_admin_ip:
					style = 'line-type:directed'
					left = 'entities:%s' % device.admin_ip
					right = 'entities:%s' % reporter_admin_ip
					
					middle_labels = []
					if igmp.age:
						middle_labels.append({'label': secs_to_str(igmp.age) + " old", 'level': 1, 'style': 'font-size:x-small'})
					middle_labels.append({'label': igmp.group, 'level': 2, 'style': 'font-size:x-small'})
					add_relation(data, left, right, style, middle_labels = middle_labels, predicate = "options.igmp")
		
	def __add_network_ips(self, data, devices):
		rows = []
		for device in devices:
			for interface in device.interfaces:
				if interface.active and interface.name and interface.ip != '127.0.0.1':
					row = []
					
					# Device
					row.append(cgi.escape(device.name))
					
					# Name
					row.append(cgi.escape(interface.name))
					
					# IP Address
					ip = interface.ip
					if interface.net_mask:
						subnet = mask_to_subnet(interface.net_mask)
						ip = '%s/%s' % (interface.ip, subnet)
						
					if interface.ip == device.admin_ip:
						row.append('<strong>%s</strong>' % ip)
					elif interface.ip == None:
						row.append(' ')
					else:
						row.append(ip)
					
					# Mac Address
					row.append(interface.mac_addr)
					
					# Speed
					if interface.active and interface.speed:
						speed = interface.speed
						if speed:
							speed = speed/1000000
							speed = '%.1f Mbps' % speed
					else:
						speed = ''
					row.append(speed)
					
					# MTU
					row.append(add_units(interface.mtu, 'B'))
					
					# Alias
					row.append(interface.alias or ' ')
					
					rows.append(row)
			
		if rows:
			detail = {}
			detail['style'] = 'html'
			detail['header'] = ['Device', 'Name', 'IP Address', 'Mac Address', 'Speed', 'MTU', 'Alias']
			
			rows = sorted(rows, key = lambda row: row[0])
			rows = sorted(rows, key = lambda row: row[1])
			detail['rows'] = rows
			
			target = 'entities:network'
			add_details(data, target, 'Interfaces', [detail], opened = 'yes', sort_key = 'alpha', key = 'ips table')
			
	# Note that both OSPF and PIM timers can differ across the network (tho they should be the
	# same between peers). Figuring out whether a difference is actually an error is complex
	# (especially with the different ospfIfType's) so we'll just list what everything is using.
	def __add_network_pim(self, data, devices):
		def add_pim_hello_detail(devices, details):
			ranges = {}	# value => [device + ifname]
			opened = False
			
			for device in devices:
				for (ifindex, value) in device.pim_hellos.items():
					interface = device.find_ifindex(ifindex)
					if interface.active:
						if interface and interface.name:
							name = device.name + ' ' + interface.name.replace('/', ' ')
						else:
							name = device.name + ' ' + ifindex
						ranges.setdefault(value, []).append(name)
			
			if len(ranges) == 1:
				details.append('All devices are using pim hello interval %ss.' % ranges.keys()[0])
			elif len(ranges) > 1:
				for (value, names) in ranges.items():
					are = ['are', 'is'][len(names) == 1]
					details.append('**%s** %s using pim hello interval %ss.' % (', '.join(names), are, value))
					opened = True
					
			return opened
			
		def add_pim_rps(devices, details):
			entries = {}	# rp name => [device name]
			opened = False
			
			for device in devices:
				for rp in device.pim_rps:
					name = self.device_name(devices, None, rp, 'name-only')
					if name:
						entries.setdefault(name, []).append(device.name)
					else:
						entries.setdefault(rp, []).append(device.name)
			
			if len(entries) == 1:
				details.append('All devices think %s is the Rendezvous Point.' % entries.keys()[0])
			elif len(entries) > 1:
				for (value, names) in entries.items():
					think = ['think', 'thinks'][len(names) == 1]
					details.append('**%s** %s the Rendezvous Point is %s.' % (', '.join(names), think, value))
					opened = True
					
			return opened
			
		def add_pim_bsrs(devices, details):
			entries = {}	# bsr name => [device name]
			opened = False
			
			for device in devices:
				for bsr in device.pim_bsrs:
					name = self.device_name(devices, None, bsr, 'name-only')
					if name:
						entries.setdefault(name, []).append(device.name)
					else:
						entries.setdefault(bsr, []).append(device.name)
			
			if len(entries) == 1:
				details.append('All devices think %s is the Boot-Strap Router.' % entries.keys()[0])
			elif len(entries) > 1:
				for (value, names) in entries.items():
					think = ['think', 'thinks'][len(names) == 1]
					details.append('**%s** %s the Boot-Strap Router is %s.' % (', '.join(names), think, value))
					opened = True
					
			return opened
			
		opened = 'no'
		
		details = []
		if add_pim_hello_detail(devices, details):
			opened = 'yes'
		if add_pim_rps(devices, details):
			opened = 'yes'
		if add_pim_bsrs(devices, details):
			opened = 'yes'
		
		if details:
			target = 'entities:network'
			add_details(data, target, 'PIM', details, opened = opened, sort_key = 'whiskey', key = 'pim')
	
	def __add_network_ospf(self, data, devices):
		def add_ospf_hello_detail(devices, details):
			ranges = {}	# value => [device + ifname]
			opened = False
			
			for device in devices:
				for (ip, value) in device.ospf_hellos.items():
					interface = device.find_ip(ip)
					if interface and interface.active:
						if interface and interface.name:
							name = device.name + ' ' + interface.name.replace('/', ' ')
						else:
							name = device.name + ' ' + ip
						ranges.setdefault(value, []).append(name)
			
			if len(ranges) == 1:
				details.append('All devices are using ospf hello interval %ss.' % ranges.keys()[0])
			elif len(ranges) > 1:
				for (value, names) in ranges.items():
					are = ['are', 'is'][len(names) == 1]
					details.append('**%s** %s using ospf hello interval %ss.' % (', '.join(names), are, value))
					opened = True
					
			return opened
			
		def add_ospf_dead_detail(devices, details):
			ranges = {}	# value => [device + ifname]
			opened = False
			
			for device in devices:
				for (ip, value) in device.ospf_deads.items():
					interface = device.find_ip(ip)
					if interface and interface.active:
						if interface and interface.name:
							name = device.name + ' ' + interface.name.replace('/', ' ')
						else:
							name = device.name + ' ' + ip
						ranges.setdefault(value, []).append(name)
			
			if len(ranges) == 1:
				details.append('All devices are using ospf dead interval %ss.' % ranges.keys()[0])
			elif len(ranges) > 1:
				for (value, names) in ranges.items():
					are = ['are', 'is'][len(names) == 1]
					details.append('**%s** %s using ospf dead interval %ss.' % (', '.join(names), are, value))
					opened = True
					
			return opened
			
		opened = 'no'
		
		details = []
		if add_ospf_hello_detail(devices, details):
			opened = 'yes'
		if add_ospf_dead_detail(devices, details):
			opened = 'yes'
		
		if details:
			target = 'entities:network'
			add_details(data, target, 'OSPF', details, opened = opened, sort_key = 'x-ray', key = 'ospf')
		
	def __add_network_mroutes(self, data, devices):
		rows = []
		for device in devices:
			for route in device.mroutes:
				if route.source != '0.0.0.0':
					row = []
					
					# Group
					row.append(route.group)
					
					# Source
					row.append(self.device_name(devices, device, route.source))
					
					# Upstream
					if route.upstream == '0.0.0.0':
						row.append('')
					else:
						row.append(self.device_name(devices, device, route.upstream))
					
					# Router
					row.append(self.device_name(devices, device, route.admin_ip))
						
					# Protocol
					row.append(cgi.escape(route.protocol))
					
					# Uptime
					row.append(cgi.escape(route.uptime))
					
					# Packets
					if route.packets != None:
						packets = self.__process_sample(device, data, {'key': '%s-%s-%s-packets' % (route.admin_ip, route.group, route.source), 'raw': route.packets, 'units': 'pps'})
						row.append(packets['html'])
					else:
						row.append('')
					
					# Octets
					if route.octets != None:
						octets = self.__process_sample(device, data, {'key': '%s-%s-%s-octets' % (route.admin_ip, route.group, route.source), 'raw': 8*route.octets/1000, 'units': 'kbps'})
						row.append(octets['html'])
					else:
						row.append('')
					
					rows.append(row)
			
		if rows:
			detail = {}
			detail['style'] = 'html'
			detail['header'] = ['Group', 'Source', 'Upstream', 'Router', 'Protocol', 'Uptime', 'Packets (pps)', 'Octets (kbps)']
			
			rows = sorted(rows, key = lambda row: row[0])	# sorted is stable and relies on Timsort so is efficient to do multiple times
			rows = sorted(rows, key = lambda row: row[1])
			rows = sorted(rows, key = lambda row: row[2])
			detail['rows'] = rows
			
			target = 'entities:network'
			footnote = '*The shaded area in the sparklines is the inter-quartile range: the range in which half the samples appear.*'
			add_details(data, target, 'MRoutes', [detail, footnote], opened = 'no', sort_key = 'gamma', key = 'mroutes table')
		
	def __add_interfaces_table(self, data, device):
		rows = []
		for interface in device.interfaces:
			if interface.name:
				name = cgi.escape(interface.name)
				
				ip = interface.ip
				if interface.net_mask:
					subnet = mask_to_subnet(interface.net_mask)
					ip = '%s/%s' % (ip, subnet)
				if interface.ip == device.admin_ip:
					ip = '<strong>%s</strong>' % ip
				elif interface.ip == None:
					ip = ' '
				
				# We always need to add samples so that they stay in sync with one another.
				if interface.in_octets != None:
					in_octets = self.__process_sample(device, data, {'key': '%s-%s-in_octets' % (device.admin_ip, name), 'raw': 8*interface.in_octets/1000, 'units': 'kbps'})
					in_cell = in_octets['html']
				else:
					in_cell = ''
				if interface.out_octets != None:
					out_octets = self.__process_sample(device, data, {'key':  '%s-%s-out_octets' % (device.admin_ip, name), 'raw': 8*interface.out_octets/1000, 'units': 'kbps'})
					out_cell = out_octets['html']
				else:
					out_cell = ''
				
				if interface.active and interface.speed:
					speed = interface.speed
					if speed:
						if out_octets and out_octets['value']:
							self.__add_interface_gauge(data, device.admin_ip, name, out_octets['value'], speed/1000)
						speed = speed/1000000
						speed = '%.1f Mbps' % speed
				else:
					speed = ''
				
				if interface.active:
					rows.append([name, ip, interface.mac_addr, speed, add_units(interface.mtu, 'B'), in_cell, out_cell])
			
		if rows:
			detail = {}
			detail['style'] = 'html'
			detail['header'] = ['Name', 'IP Address', 'Mac Address', 'Speed', 'MTU', 'In Octets (kbps)', 'Out Octets (kbps)']
			detail['rows'] = sorted(rows, key = lambda row: row[0])
			
			target = 'entities:%s' % device.admin_ip
			footnote = '*The shaded area in the sparklines is the inter-quartile range: the range in which half the samples appear.*'
			add_details(data, target, 'Interfaces', [detail, footnote], opened = 'yes', sort_key = 'alpha', key = 'interfaces table')
			
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
	# 4) The html link is returned to our caller.
	def __process_sample(self, device, data, table):
		# on input table should have: key, raw, and units
		# on input html and value are added
		table['html'] = ''
		value = 0.0
		if self.__last_time and self.__last_sample.get(table['key'], 0.0) > 0.0:
			elapsed = self.__current_time - self.__last_time
			if elapsed > 1.0:
				value = (table['raw'] - self.__last_sample[table['key']])/elapsed
		table['value'] = value
		if self.__num_updates >= 2:
			data['samples'].append({'name': table['key'], 'value': value, 'units': table['units']})
		
		# When dynamically adding html content browsers will not reload images that have
		# been already loaded. To work around this we add a unique fragment identifier
		# which the server will ignore.
		if self.__num_updates >= 2:
			url = '/generated/%s.png#%s' % (table['key'], self.__num_updates)
			table['html'] = "<img src = '%s' alt = '%s'>" % (url, table['key'])
		
		self.__last_sample[table['key']] = table['raw']
		return table
	
# Parse command line.
parser = argparse.ArgumentParser(description = "Uses snmp and/or ssh to model a network and sends the result to a gnos server.")
parser.add_argument("--dont-put", dest = 'put', action='store_false', default=True, help = 'log results instead of PUTing them')
parser.add_argument("--duration", action='store', default=float('inf'), type=float, metavar='SECS', help = 'amount of time to poll (for testing)')
parser.add_argument("--ip", dest = 'ip', metavar = "IP", required=True, help = 'IP address to send PUTs to')
parser.add_argument("--port", dest = 'port', metavar = "PORT", required=True, help = 'port to send PUTs to')
parser.add_argument("--stdout", action='store_true', default=False, help = 'log to stdout instead of snmp-modeler.log')
parser.add_argument("--verbose", "-v", action='count', help = 'print extra information')
parser.add_argument("--version", "-V", action='version', version='%(prog)s 0.2')	# TODO: keep this version synced up with the gnos version
parser.add_argument("config", metavar = "CONFIG-FILE", help = "path to json formatted configuration file")
env.options = parser.parse_args()

# Configure logging.
configure_logging(env.options.stdout, 'net-modeler.log')

try:
	# Read config info.
	with open(env.options.config, 'r') as f:
		env.config = json.load(f)
		
	env.config['server'] = env.options.ip
	env.config['port'] = env.options.port
	
	poller = Poll()
	poller.run()
except:
	env.logger.error("net-modeler failed", exc_info = True)
