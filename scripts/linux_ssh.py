#!/usr/bin/python
# Modeler designed for use on Linux devices that are not running SNMP. It's
# designed to be fairly minimal (if you want detailed information then get
# SNMP running).
import cgi, json, itertools, httplib, re, subprocess, sys, threading, time
from helpers import *
from net_types import *

connection = None

def find_index(line, needle):
	parts = line.split()
	for i in xrange(0, len(parts)):
		if parts[i].startswith(needle):
			return i
	return None

def subnet_to_mask(s):
	count = int(s)
	
	result = 0
	bit = 1 << 31
	while count > 0:
		result |= bit
		bit >>= 1
		count -= 1
	
	return int_to_ip(result)

class UName(object):
	def command(self):
		return 'uname -a'
	
	# Linux auto-fat 2.6.32-33-server #70-Ubuntu SMP Thu Jul 7 22:28:30 UTC 2011 x86_64 GNU/Linux
	def process(self, data, text, query):
		env.logger.debug("uname: '%s'" % text)
		query.device.system_info += text + "\n"
		
class Uptime(object):
	def command(self):
		return 'uptime'
		
	up_expr1 = re.compile(r'up \s+ (\d+) \s+ (min|sec|day)', re.MULTILINE | re.VERBOSE)
	up_expr2 = re.compile(r'up \s+ (\d+) : (\d+) ,', re.MULTILINE | re.VERBOSE)
	load_expr = re.compile(r'load\ average: \s+ ([\d.]+)', re.MULTILINE | re.VERBOSE)
	
	# 04:43:21 up 0 min, load average: 1.02, 0.27, 0.09
	# 19:08:55 up  9:39, load average: 0.00, 0.01, 0.04
	# 07:13:42 up 52 days, 16:04,  3 users,  load average: 0.00, 0.00, 0.05
	def process(self, data, text, query):
		env.logger.debug("uptime: '%s'" % text)
		target = 'entities:%s' % query.device.admin_ip
		
		match1 = re.search(Uptime.up_expr1, text)
		match2 = re.search(Uptime.up_expr2, text)
		if match1:
			if match1.group(2) == 'sec':
				query.device.uptime = float(match1.group(1))
			elif match1.group(2) == 'min':
				query.device.uptime = 60*float(match1.group(1))
			elif match1.group(2) == 'day':
				query.device.uptime = 86400*float(match1.group(1))
		elif match2:
			query.device.uptime = 60*60*float(match2.group(1)) + 60*float(match2.group(2))
		else:
			env.logger.error("Failed to match uptime: %s" % text)
			
		# The load average is an average of the number of processes forced to wait
		# for CPU over the last 1, 5, and 15 minutes. We'll record the average for the 
		# last minute so that we can compute processor load after we know how many
		# cores there are.
		match = re.search(Uptime.load_expr, text)
		if match:
			query.load_average = float(match.group(1))
		
class CpuInfo(object):
	# Note that this will count both CPUs and cores.
	def command(self):
		return 'cat /proc/cpuinfo | grep -E "[Pp]rocessor[^:alpha:]+:" | wc -l'
		
	# 1
	def process(self, data, text, query):
		env.logger.debug("cpuinfo: '%s'" % text)
		if text.isdigit():
			query.num_cores = int(text)
		
class Df(object):
	def command(self):
		return 'df -h'
		
	# Filesystem                Size      Used Available Use% Mounted on
	# /dev/root                 6.6M      6.6M         0 100% /rom
	# tmpfs                    30.5M     60.0K     30.5M   0% /tmp
	# tmpfs                   512.0K         0    512.0K   0% /dev
	# /dev/mtdblock3            7.3M    724.0K      6.5M  10% /overlay
	# overlayfs:/overlay        7.3M    724.0K      6.5M  10% /
	def process(self, data, text, query):
		lines = text.splitlines()
		env.logger.debug("df: '%s'" % lines)
		
		use_index = find_index(lines[0], "Use%")
		mount_index = find_index(lines[0], "Mount")
		if use_index and mount_index:
			target = 'entities:%s' % query.device.admin_ip
			for line in lines[1:]:
				self.__process_line(data, target, line, use_index, mount_index)
				
	def __process_line(self, data, target, line, use_index, mount_index):
		parts = line.split()
		if parts[mount_index] != '/rom':
			value = int(parts[use_index][:-1])/100.0
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
				add_gauge(data, target, parts[mount_index], value, level, style, sort_key = 'zzz')
		
class IpAddress(object):
	def command(self):
		return '/usr/sbin/ip -o address show'
		
	# 1: lo: <LOOPBACK,UP,LOWER_UP> mtu 16436 qdisc noqueue state UNKNOWN \    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00
	# 1: lo    inet 127.0.0.1/8 brd 127.255.255.255 scope host lo
	# 2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc pfifo_fast state UNKNOWN qlen 100\    link/ether 00:08:a2:04:15:4e brd ff:ff:ff:ff:ff:ff
	# 2: eth0    inet 10.102.0.10/24 brd 10.102.0.255 scope global eth0
	# 3: eth1: <BROADCAST,MULTICAST> mtu 1500 qdisc noop state DOWN qlen 100\    link/ether 00:08:a2:04:15:4f brd ff:ff:ff:ff:ff:ff
	# 5: gre0: <NOARP> mtu 1476 qdisc noop state DOWN \    link/gre 0.0.0.0 brd 0.0.0.0
	def process(self, data, text, query):
		lines = text.splitlines()
		env.logger.debug("ip: '%s'" % lines)
		
		# This is pretty crappy. Not sure if there's a way to get more machine readable information
		# (other than snmp).
		for line in lines:
			interface = self.__find_interface(query.device, line)
			if interface:
				self.__set_name(interface, line)
				self.__set_status(interface, line)
				self.__set_mtu(interface, line)
				self.__set_ip(interface, line)
				self.__set_mac(interface, line)
			
	def __set_name(self, interface, line):
		parts = line.split()
		name = parts[1]
		if name[-1] == ':':
			name = name[:-1]
		interface.name = name
	
	def __set_status(self, interface, line):
		if '<UP,' in line or ',UP,' in line or ',UP>' in line:
			interface.status = 'up'
		
	def __set_mtu(self, interface, line):
		parts = line.split()
		if 'mtu' in parts:
			i = parts.index('mtu')
			interface.mtu = int(parts[i+1])
			
	def __set_ip(self, interface, line):
		parts = line.split()
		if 'inet' in parts:
			i = parts.index('inet')
			pieces = parts[i+1].split('/')
			if len(pieces) == 1:
				interface.ip = pieces[0]
			elif len(pieces) == 2:
				interface.ip = pieces[0]
				interface.net_mask = subnet_to_mask(pieces[1])
			
	def __set_mac(self, interface, line):
		parts = line.split()
		if 'link/ether' in parts:
			i = parts.index('link/ether')
			interface.mac_addr = parts[i+1]
		else:
			interface.mac_addr = ''
	
	def __find_interface(self, device, line):
		i = line.find(':')
		if i:
			n = line[:i]
			if n.isdigit():
				for candidate in device.interfaces:
					if candidate.index == n:
						return candidate
				
				interface = Interface()
				interface.admin_ip = device.admin_ip
				interface.index = n
				interface.status = 'down'
				device.interfaces.append(interface)
				return interface
		return None
		
class Netstat(object):
	def command(self):
		return 'netstat -rn'
		
	# Kernel IP routing table
	# Destination     Gateway         Genmask         Flags   MSS Window  irtt Iface
	# 10.103.0.0      0.0.0.0         255.255.255.0   U         0 0          0 eth0
	# 0.0.0.0         10.103.0.2      0.0.0.0         UG        0 0          0 eth0
	def process(self, data, text, query):
		lines = text.splitlines()
		env.logger.debug("netstat: '%s'" % lines)
		
		if find_index(lines[1], "Gateway"):
			for line in lines[2:]:
				self.__process_line(query.device, lines[1], line)
	
	def __process_line(self, device, header, line):
		route = Route()
		route.via_ip = self.__get(header, line, 'Gateway')
		route.dst_subnet = self.__get(header, line, 'Destination')
		route.dst_mask = self.__get(header, line, 'Genmask')
		route.protocol = 'local'
		route.metric = 1
		route.ifindex = self.__find_interface(device, self.__get(header, line, 'Iface'))
		device.routes.append(route)
		
	def __get(self, header, line, name):
		i = find_index(header, name)
		if i != None:
			return line.split()[i]
		return None
		
	# Assumes IpAddress has already run.
	def __find_interface(self, device, ifname):
		if ifname:
			for candidate in device.interfaces:
				if candidate.name == ifname:
					return candidate.index
		return None
		
# TODO:
# add interface table, use: /usr/sbin/ip address show
# add interface stats, use: /usr/sbin/ip -s  link (netstat -i would be nicer, but not always available)
# add routing table

class DeviceRunner(object):
	def __init__(self, ip, ssh, handlers):
		self.ip = ip
		self.__ssh = ssh
		self.__handlers = handlers
		commands = map(lambda x: '%s' % x.command(), handlers)
		self.__command = '; echo \'^^^\'; '.join(commands)
		
	def run(self):
		self.results = None
		command = '%s%s "%s"' % (self.__ssh, self.ip, self.__command)
		env.logger.debug(command)
		try:
			result = run_process(command)
			parts = result.split('^^^')
			if len(parts) == len(self.__handlers):
				self.results = map(lambda x: x.strip(), parts)
			else:
				raise Exception("Expected %s results but found '%s'" % (len(self.__handlers), result))
		except:
			env.logger.error("Error executing `%s`" % command, exc_info = True)

class QueryDevice(object):
	def __init__(self, device):
		self.__handlers = [UName(), Uptime(), CpuInfo(), Df(), IpAddress(), Netstat()]
		self.device = device
		self.load_average = None		# 1 min load average
		self.num_cores = None
	
	def run(self):
		self.__runner = DeviceRunner(self.device.admin_ip, self.device.config['ssh'], self.__handlers)
		self.__runner.run()
		
	def process(self, data):
		target = 'entities:%s' % self.__runner.ip
		if self.__runner.results:
			close_alert(data, target, key = 'device down')
			assert len(self.__runner.results) == len(self.__handlers)
			for i in xrange(0, len(self.__runner.results)):
				try:
					self.__handlers[i].process(data, self.__runner.results[i], self)
				except:
					env.logger.error("Failed to process %s" % self.__handlers[i].command, exc_info=True)
		else:
			open_alert(data, target, key = 'device down', mesg = 'Device is down.', resolution = 'Check the power cable, power it on if it is off, check the IP address, verify routing.', kind = 'error')
		
		self.__add_cpu_load_gauge(data)
		
	def __add_cpu_load_gauge(self, data):
		if self.load_average != None and self.num_cores != None:
			value = self.load_average/self.num_cores
			target = 'entities:%s' % self.device.admin_ip
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
