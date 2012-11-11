#!/usr/bin/python
# Collects information about a network using snmp and ssh. Ships the results off to gnos using json.
import cgi, httplib, json, sys, time		# , , itertools, , re, , threading, 
import linux_ssh, snmp
from base_modeler import *
from net_types import *

try:
	import argparse
except:
	sys.stderr.write("This script requires Python 2.7 or later\n")
	sys.exit(2)

def ip_to_int(ip):
	parts = ip.split('.')
	if len(parts) != 4:
		raise Exception("expected an IP address but found: '%s'" % ip)
	return (int(parts[0]) << 24) | (int(parts[1]) << 16) | (int(parts[2]) << 8) | int(parts[3])

def int_to_ip(value):
	return '%s.%s.%s.%s' % ((value << 24) & 0xFF, (value << 16) & 0xFF, (value << 8) & 0xFF, value & 0xFF)

def add_units(value, unit):
	if value or type(value) == float:
		value = '%s %s' % (value, unit)
	return value

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
	
	def run(self):
		try:
			if self.__connection:
				send_entities(self.__connection)
			
			rate = env.config['poll-rate']
			while True:
				self.__current_time = time.time()
				if not env.options.put:
					env.logger.info("-" * 60)
					
				devices = map(lambda d: Device(d), env.config['devices'].values())
				data = {'modeler': 'net', 'entities': [], 'relations': [], 'labels': [], 'gauges': [], 'details': [], 'alerts': [], 'samples': [], 'charts': []}
				for device in devices:
					self.__query_device(data, device)
				self.__update_routes(devices)
				
				for device in devices:
					self.__process_device(data, device)
				self.__add_next_hop_relations(data, devices)
				self.__add_selection_route_relations(data, devices)
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
				
	def __query_device(self, data, device):
		if device.config['type'] == 'linux_ssh':
			query = linux_ssh.QueryDevice(device)
		elif device.config['type'] == 'snmp':
			query = snmp.QueryDevice(device)
		else:
			env.logger.error("bad modeler: %s" % device.config['modeler'])
			
		if query:
			query.run(data, self.__num_updates)
			
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
			
		def admin_ip_by_subnet(devices, src_device, network_ip, netmask):
			# First try the devices we don't know about (these are the devices we want to use when 
			# forwarding to a device on an attached subnet).
			subnet = ip_to_int(network_ip) & netmask
			for (name, device) in env.config["devices"].items():
				if device['type'] != 'snmp':		# TODO: add linux_ssh if it ever collects Interface info
					candidate = ip_to_int(device['ip']) & netmask
					if candidate == subnet:
						return device['ip']
			
			# Then try to find a device we do know about that isn't src_admin.
			# This will tend to be the direct link to a peer machine case.
			for device in devices:
				for interface in device.interfaces:
					if interface.ip:
						candidate = ip_to_int(interface.ip) & netmask
						if candidate == subnet and device.admin_ip != src_device.admin_ip:
							return device.admin_ip
			
			return None
			
		for device in devices:
			for route in device.routes:
				route.src_interface = interface_by_index(device, route.ifindex)
				if route.via_ip != None and route.via_ip != '0.0.0.0':
					route.via_interface = interface_by_device_ip(devices, route.via_ip)
				if route.dst_subnet:
					route.dst_admin_ip = admin_ip_by_subnet(devices, device, route.dst_subnet, ip_to_int(route.dst_mask))
		
	def __process_device(self, data, device):
		# admin ip label
		target = 'entities:%s' % device.admin_ip
		add_label(data, target, device.admin_ip, 'a', level = 1, style = 'font-size:small')
		
		if device.uptime:
			# uptime label
			key = 'alpha'		# want these to appear before most other labels
			add_label(data, target, 'uptime: %s' % secs_to_str(device.uptime), key, level = 2, style = 'font-size:small')
			
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
			self.__add_routing_table(data, device)
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
			
	def __add_routing_table(self, data, device):
		rows = []
		for route in device.routes:
			dest = route.dst_subnet
			subnet = get_subnet(route.dst_mask)
			dest = '%s/%s' % (dest, subnet)
			
			if route.src_interface:
				out = route.src_interface.name
			else:
				out = '?'
			
			if route.via_interface:
				via = route.via_interface.name + ' ' + route.via_ip
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
				if route.via_interface:
					# Used when forwarding (through a router or through an interface and locally to the router).
					via_admin = route.via_interface.admin_ip
					if route.dst_admin_ip and device.admin_ip != via_admin:
						key = (device.admin_ip, via_admin, route.dst_admin_ip)
						routes[key] = route
				elif route.dst_admin_ip:
					# If the netmask is all ones then this will be a direct link to a peer machine.
					# Otherwise it is used when forwarding to a device on an attached subnet.
					key = (device.admin_ip, route.dst_admin_ip, route.dst_admin_ip)
					routes[key] = route
				
		for (key, route) in routes.items():
			(src_admin, via_admin, dst_admin) = key
			left = 'entities:%s' % src_admin
			right = 'entities:%s' % via_admin
			
			if route.src_interface:
				left_label = {'label': '%s %s' % (route.src_interface.name, route.src_interface.ip), 'level': 2, 'style': 'font-size:xx-small'}
			else:
				left_label = {'label': route.src_interface.ip, 'level': 2, 'style': 'font-size:xx-small'}
			
			middle_label = {'label': '%s cost %s' % (route.protocol, route.metric), 'level': 1, 'style': 'font-size:small'}
			
			ifname = ''
			if route.via_interface and route.via_interface.name:
				ifname = route.via_interface.name + ' '
			right_label = {'label': ifname + route.via_ip, 'level': 2, 'style': 'font-size:xx-small'}
			
			predicate = "options.routes selection.name '%s' ends_with and" % dst_admin
			add_relation(data, left, right, 'line-type:directed line-color:blue line-width:3', left_label = left_label, middle_label = middle_label, right_label = right_label, predicate = predicate)
		
	def __add_next_hop_relations(self, data, devices):
		routes = {}			# (src admin ip, via admin ip) => Route
		for device in devices:
			for route in device.routes:
				if route.via_interface:
					src_admin = route.src_interface.admin_ip
					via_admin = route.via_interface.admin_ip
					if src_admin != via_admin:
						routes[(src_admin, via_admin)] = route
				elif route.dst_admin_ip:
					src_admin = route.src_interface.admin_ip
					routes[(src_admin, route.dst_admin_ip)] = route
		
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
				add_relation(data, left, right, style, middle_label = {'label': 'next hop', 'level': 1, 'style': 'font-size:small'}, predicate = predicate)
		
	def __add_interfaces_table(self, data, device):
		rows = []
		for interface in device.interfaces:
			name = cgi.escape(interface.name)
			
			ip = interface.ip
			if interface.net_mask:
				subnet = get_subnet(interface.net_mask)
				if ip == device.admin_ip:
					ip = '<strong>%s/%s</strong>' % (ip, subnet)
				else:
					ip = '%s/%s' % (ip, subnet)
			
			# We always need to add samples so that they stay in sync with one another.
			if interface.in_octets != None:
				in_octets = self.__process_sample(data, {'key': '%s-%s-in_octets' % (device.admin_ip, name), 'raw': 8*interface.in_octets/1000, 'units': 'kbps'})
			if interface.out_octets != None:
				out_octets = self.__process_sample(data, {'key':  '%s-%s-out_octets' % (device.admin_ip, name), 'raw': 8*interface.out_octets/1000, 'units': 'kbps'})
			
			if interface.active and interface.speed:
				speed = interface.speed
				if speed:
					if out_octets['value']:
						self.__add_interface_gauge(data, device.admin_ip, name, out_octets['value'], speed/1000)
					speed = speed/1000000
					speed = '%.1f Mbps' % speed
				
				rows.append([name, ip, interface.mac_addr, speed, add_units(interface.mtu, 'B'), in_octets['html'], out_octets['html']])
			
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
	# 4) The sample value and html link are returned to out caller.
	def __process_sample(self, data, table):
		# On input table has: key, raw, and units
		# On exit: value and html are added
		table['value'] = None
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
parser.add_argument("--stdout", action='store_true', default=False, help = 'log to stdout instead of snmp-modeler.log')
parser.add_argument("--verbose", "-v", action='count', help = 'print extra information')
parser.add_argument("--version", "-V", action='version', version='%(prog)s 0.1')	# TODO: keep this version synced up with the gnos version
parser.add_argument("config", metavar = "CONFIG-FILE", help = "path to json formatted configuration file")
env.options = parser.parse_args()

# Configure logging.
configure_logging(env.options.stdout, 'net-modeler.log')

# Read config info.
with open(env.options.config, 'r') as f:
	env.config = json.load(f)
	
poller = Poll()
poller.run()
